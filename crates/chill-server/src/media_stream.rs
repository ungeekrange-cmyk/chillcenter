//! Descriptor-relative file access and bounded raw Range streaming. No decoding.
use crate::{
    api::{ApiError, AppState},
    playback::{PlaybackService, StreamLease},
};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    ffi::CString,
    fs::{File, Metadata},
    hash::{Hash, Hasher},
    io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{FileExt, MetadataExt},
        },
    },
    path::{Component, Path as FsPath},
    sync::Arc,
    time::Duration,
};
use tokio::sync::OwnedSemaphorePermit;

const CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FileVersion {
    pub size: u64,
    dev: u64,
    inode: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl FileVersion {
    pub(crate) fn from_metadata(m: &Metadata) -> io::Result<Self> {
        if !m.is_file() {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Self {
            size: m.len(),
            dev: m.dev(),
            inode: m.ino(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        })
    }
    pub(crate) fn revision(&self, root_revision: u64) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        root_revision.hash(&mut h);
        (h.finish() & i64::MAX as u64).max(1)
    }
    pub(crate) fn etag(&self, root_revision: u64) -> String {
        format!("\"chill-{:016x}\"", self.revision(root_revision))
    }
}

/// No canonicalize/check-then-open race: every component is opened relative to
/// its already-open parent with O_NOFOLLOW, including the configured root.
pub(crate) fn open_indexed(root: &FsPath, path: &FsPath) -> io::Result<(File, FileVersion)> {
    if !root.is_absolute()
        || root == FsPath::new("/")
        || !path.is_absolute()
        || [root, path].iter().any(|p| {
            p.as_os_str()
                .as_bytes()
                .split(|b| *b == b'/')
                .any(|c| c == b"." || c == b"..")
        })
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|_| io::ErrorKind::PermissionDenied)?;
    if relative.as_os_str().is_empty() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut directory = File::open("/")?;
    for c in root.components() {
        if let Component::Normal(name) = c {
            directory = open_child(&directory, name, true)?;
        } else if !matches!(c, Component::RootDir) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    let components: Vec<_> = relative.components().collect();
    for (i, c) in components.iter().enumerate() {
        let Component::Normal(name) = c else {
            return Err(io::ErrorKind::PermissionDenied.into());
        };
        let file = open_child(&directory, name, i + 1 < components.len())?;
        if i + 1 == components.len() {
            let version = FileVersion::from_metadata(&file.metadata()?)?;
            return Ok((file, version));
        }
        directory = file;
    }
    Err(io::ErrorKind::PermissionDenied.into())
}
fn open_child(parent: &File, name: &std::ffi::OsStr, directory: bool) -> io::Result<File> {
    let name = CString::new(name.as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: live parent descriptor and a NUL-terminated component; read-only.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: unique ownership of the freshly returned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

pub(crate) async fn io_job<T: Send + 'static>(
    service: &PlaybackService,
    job: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> Result<T, ApiError> {
    let permit = service
        .io
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "MEDIA_BUSY"))?;
    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        job()
    });
    // A stuck kernel syscall retains its worker permit even after the caller
    // times out: never start unlimited replacements on an unavailable NAS.
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .map_err(|_| ApiError(StatusCode::GATEWAY_TIMEOUT, "MEDIA_TIMEOUT"))?
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "MEDIA_UNAVAILABLE"))?
        .map_err(|error| {
            if error.kind() == io::ErrorKind::InvalidData {
                ApiError(StatusCode::PRECONDITION_FAILED, "MEDIA_CHANGED")
            } else {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "MEDIA_UNAVAILABLE")
            }
        })
}

pub(crate) fn range(value: &str, size: u64) -> Option<(u64, u64)> {
    if size == 0 || value.len() > 100 {
        return None;
    }
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    let decimal = |s: &str| {
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            s.parse::<u64>().ok()
        } else {
            None
        }
    };
    if start.is_empty() {
        let suffix = decimal(end)?;
        if suffix == 0 {
            return None;
        }
        return Some((size.saturating_sub(suffix), size - 1));
    }
    let start = decimal(start)?;
    let end = if end.is_empty() {
        size - 1
    } else {
        decimal(end)?.min(size - 1)
    };
    (start < size && start <= end).then_some((start, end))
}

struct StreamState {
    service: PlaybackService,
    lease: StreamLease,
    file: Arc<File>,
    offset: u64,
    remaining: u64,
    _permit: OwnedSemaphorePermit,
}

pub(crate) async fn stream(
    State(state): State<AppState>,
    Path(media): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let session = one_header(&headers, "x-chill-session")?;
    let generation = one_header(&headers, "x-chill-generation")?
        .parse::<u64>()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "LEASE_INVALID"))?;
    let lease = state.playback.stream_lease(&session, generation, &media)?;
    let _permit = state
        .playback
        .streams
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "MEDIA_BUSY"))?;
    let root = lease.root.clone();
    let path = lease.path.clone();
    let (file, version) = io_job(&state.playback, move || open_indexed(&root, &path)).await?;
    if version != lease.version {
        state.playback.invalidate_stream(&lease);
        return Err(ApiError(StatusCode::PRECONDITION_FAILED, "MEDIA_CHANGED"));
    }
    state.playback.check_stream(&lease)?;
    let etag = version.etag(lease.root_revision);
    if headers.contains_key("if-match") && one_header(&headers, "if-match")? != etag {
        return Err(ApiError(StatusCode::PRECONDITION_FAILED, "MEDIA_CHANGED"));
    }
    let requested = one_header(&headers, "range")?;
    let Some((start, end)) = range(&requested, version.size) else {
        let mut response =
            ApiError(StatusCode::RANGE_NOT_SATISFIABLE, "RANGE_INVALID").into_response();
        response.headers_mut().insert(
            "content-range",
            format!("bytes */{}", version.size)
                .parse()
                .expect("numeric range"),
        );
        return Ok(response);
    };
    let length = end - start + 1;
    let work = StreamState {
        service: state.playback,
        lease,
        file: Arc::new(file),
        offset: start,
        remaining: length,
        _permit,
    };
    let chunks = futures_util::stream::unfold(work, |mut s| async move {
        if s.remaining == 0 {
            return None;
        }
        let result = async {
            s.service
                .check_stream(&s.lease)
                .map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))?;
            let file = s.file.clone();
            let expected = s.lease.version.clone();
            let offset = s.offset;
            let length = (s.remaining.min(CHUNK_BYTES as u64)) as usize;
            let bytes = io_job(&s.service, move || {
                if FileVersion::from_metadata(&file.metadata()?)? != expected {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let mut data = vec![0; length];
                let count = file.read_at(&mut data, offset)?;
                if count == 0 {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
                data.truncate(count);
                if FileVersion::from_metadata(&file.metadata()?)? != expected {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                Ok(data)
            })
            .await
            .map_err(|error| {
                if error.1 == "MEDIA_CHANGED" {
                    s.service.invalidate_stream(&s.lease);
                }
                io::Error::from(io::ErrorKind::Other)
            })?;
            s.service
                .check_stream(&s.lease)
                .map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))?;
            Ok::<_, io::Error>(bytes)
        }
        .await;
        match result {
            Ok(bytes) => {
                s.offset += bytes.len() as u64;
                s.remaining -= bytes.len() as u64;
                Some((Ok(Bytes::from(bytes)), s))
            }
            Err(e) => {
                s.remaining = 0;
                Some((Err(e), s))
            }
        }
    });
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("content-type", "application/octet-stream")
        .header("accept-ranges", "bytes")
        .header(
            "content-range",
            format!("bytes {start}-{end}/{}", version.size),
        )
        .header("content-length", length.to_string())
        .header("etag", etag)
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .body(Body::from_stream(chunks))
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "STREAM_UNAVAILABLE"))
}
fn one_header(headers: &HeaderMap, key: &str) -> Result<String, ApiError> {
    let all = headers.get_all(key);
    let mut values = all.iter();
    let value = values
        .next()
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "STREAM_HEADERS_REQUIRED"))?;
    if values.next().is_some() || value.len() > 128 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "STREAM_HEADERS_INVALID"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_boundaries() {
        assert_eq!(range("bytes=0-3", 10), Some((0, 3)));
        assert_eq!(range("bytes=8-99", 10), Some((8, 9)));
        assert_eq!(range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(range("bytes=5-", 10), Some((5, 9)));
        for bad in [
            "bytes=10-",
            "bytes=4-2",
            "bytes=-0",
            "bytes=0-1,3-4",
            "bytes=+1-3",
            "units=0-2",
        ] {
            assert_eq!(range(bad, 10), None);
        }
    }
    #[test]
    fn descriptors_refuse_symlinks_escape_and_special_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::write(root.join("video.mp4"), b"movie").unwrap();
        let (file, version) = open_indexed(&root, &root.join("video.mp4")).unwrap();
        assert_eq!(version.size, 5);
        drop(file);
        std::os::unix::fs::symlink("video.mp4", root.join("alias.mp4")).unwrap();
        assert!(open_indexed(&root, &root.join("alias.mp4")).is_err());
        std::os::unix::fs::symlink(&root, root.join("dir-link")).unwrap();
        assert!(open_indexed(&root, &root.join("dir-link/video.mp4")).is_err());
        assert!(open_indexed(&root, &root.join("../outside.mp4")).is_err());
        assert!(open_indexed(&root, &root.join("./video.mp4")).is_err());
        assert!(open_indexed(&root, FsPath::new("/etc/passwd")).is_err());
        assert!(open_indexed(&root, &root).is_err());
        std::fs::create_dir(root.join("not-video.mp4")).unwrap();
        assert!(open_indexed(&root, &root.join("not-video.mp4")).is_err());
        let fifo = CString::new(root.join("pipe.mp4").as_os_str().as_bytes()).unwrap();
        // SAFETY: a NUL-terminated, fresh fixture path; no production filesystem.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(open_indexed(&root, &root.join("pipe.mp4")).is_err());
    }
}
