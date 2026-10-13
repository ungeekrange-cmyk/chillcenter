//! Descriptor-relative file access and bounded raw Range streaming. No decoding.
use crate::{
    api::{ApiError, AppState},
    playback::{PlaybackService, StreamLease},
};
use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    ffi::CString,
    fs::{File, Metadata},
    hash::{Hash, Hasher},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{FileExt, MetadataExt},
        },
    },
    path::{Component, Path as FsPath},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::OwnedSemaphorePermit;

const CHUNK_BYTES: usize = 64 * 1024;
const MAX_SUBTITLE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(serde::Serialize)]
pub(crate) struct SubtitleCandidate { id:String, title:String, language:Option<String>, ambiguous:bool, unavailable:bool }
#[derive(serde::Deserialize)] pub(crate) struct SubtitleQuery { pub session:String, pub generation:u64 }

fn subtitle_files(root:&FsPath, video:&FsPath)->io::Result<Vec<(String,std::path::PathBuf,FileVersion)>> {
    let stem=video.file_stem().and_then(|v|v.to_str()).ok_or(io::ErrorKind::InvalidInput)?;
    let dir=video.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let mut out: Vec<(String, std::path::PathBuf,FileVersion)>=Vec::new();
    for entry in std::fs::read_dir(dir)? { let entry=entry?; let path=entry.path();
        let Some(name)=path.file_name().and_then(|v|v.to_str()) else {continue};
        let valid=["srt","vtt","ass","ssa"].iter().any(|e|name.to_ascii_lowercase().ends_with(&format!(".{e}")));
        if valid && (name==format!("{stem}.srt") || name.starts_with(&format!("{stem}."))) && let Ok((_,v))=open_indexed(root,&path) && v.size<=MAX_SUBTITLE_BYTES { out.push((name.into(),path,v)); }
    }
    out.sort_by(|a,b|a.0.cmp(&b.0)); Ok(out)
}
fn subtitle_id(name:&str, version:&FileVersion)->String { let mut h=std::collections::hash_map::DefaultHasher::new(); name.hash(&mut h);version.hash(&mut h);format!("sub-{:016x}",h.finish()) }
fn language(name:&str, stem:&str)->Option<String>{ let rest=name.strip_prefix(stem)?.strip_prefix('.')?; let token=rest.split('.').next()?; (2..=16).contains(&token.len()).then(||token.to_ascii_lowercase()) }
pub(crate) async fn subtitles(State(state):State<AppState>,Path((profile,media)):Path<(String,String)>,Query(q):Query<SubtitleQuery>)->Result<axum::Json<serde_json::Value>,ApiError>{
 let lease=state.playback.stream_lease_for_profile(&profile,&q.session,q.generation,&media)?; let root=lease.root.clone(); let path=lease.path.clone();
 let files=io_job(&state.playback,move||subtitle_files(&root,&path)).await?; state.playback.check_stream(&lease)?;
 let stem=lease.path.file_stem().and_then(|v|v.to_str()).unwrap_or("").to_owned(); let ambiguous=files.len()>1;
 Ok(axum::Json(serde_json::json!({"items":files.into_iter().map(|(title,_,version)|SubtitleCandidate{id:subtitle_id(&title,&version),language:language(&title,&stem),title,ambiguous,unavailable:false}).collect::<Vec<_>>() }))) }
pub(crate) async fn subtitle(State(state):State<AppState>,Path((profile,media,id)):Path<(String,String,String)>,Query(q):Query<SubtitleQuery>,headers:HeaderMap)->Result<Response,ApiError>{
 let lease=state.playback.stream_lease_for_profile(&profile,&q.session,q.generation,&media)?; let root=lease.root.clone();let video=lease.path.clone(); let requested=id.clone();
 let (path,expected)=io_job(&state.playback,move||subtitle_files(&root,&video).and_then(|v|v.into_iter().find(|(name,_,version)|subtitle_id(name,version)==requested).map(|(_,path,version)|(path,version)).ok_or_else(||io::Error::from(io::ErrorKind::NotFound)))).await?;
 let root=lease.root.clone(); let (file,version)=io_job(&state.playback,move||open_indexed(&root,&path)).await?; if version!=expected{return Err(ApiError(StatusCode::PRECONDITION_FAILED,"SUBTITLE_CHANGED"))}; if version.size>MAX_SUBTITLE_BYTES{return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE,"SUBTITLE_TOO_LARGE"))}; state.playback.check_stream(&lease)?;
 let etag=version.etag(lease.root_revision); if headers.contains_key("if-match") && one_header(&headers,"if-match")?!=etag{return Err(ApiError(StatusCode::PRECONDITION_FAILED,"MEDIA_CHANGED"))};
 let bytes=io_job(&state.playback,move||{let mut v=Vec::with_capacity(version.size as usize);let f=file;f.take(MAX_SUBTITLE_BYTES+1).read_to_end(&mut v)?; if v.len() as u64>MAX_SUBTITLE_BYTES || v.iter().take(4096).any(|b|*b==0) || std::str::from_utf8(&v).is_err(){return Err(io::ErrorKind::InvalidData.into())};Ok(v)}).await?;
 Response::builder().status(StatusCode::OK).header("content-type","text/plain; charset=utf-8").header("content-length",bytes.len()).header("etag",etag).header("cache-control","no-store").header("x-content-type-options","nosniff").body(Body::from(bytes)).map_err(|_|ApiError(StatusCode::INTERNAL_SERVER_ERROR,"STREAM_UNAVAILABLE"))
}

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
    guard: Arc<StreamGuard>,
}

/// Capacity ownership can be revoked without polling a backpressured body.
/// A revoked body can never send new chunks even if its consumer resumes.
struct StreamGuard {
    permit: Mutex<Option<OwnedSemaphorePermit>>,
    cancelled: AtomicBool,
    last_poll: Mutex<Instant>,
}
impl StreamGuard {
    fn new(
        service: PlaybackService,
        lease: StreamLease,
        permit: OwnedSemaphorePermit,
    ) -> Arc<Self> {
        let guard = Arc::new(Self {
            permit: Mutex::new(Some(permit)),
            cancelled: AtomicBool::new(false),
            last_poll: Mutex::new(Instant::now()),
        });
        let weak = Arc::downgrade(&guard);
        tokio::spawn(async move {
            let mut ticks = tokio::time::interval(Duration::from_millis(100));
            loop {
                ticks.tick().await;
                let Some(guard) = weak.upgrade() else {
                    break;
                };
                let idle = guard
                    .last_poll
                    .lock()
                    .map(|at| at.elapsed() >= Duration::from_secs(5))
                    .unwrap_or(true);
                if idle || service.check_stream(&lease).is_err() {
                    guard.cancel();
                    break;
                }
                if guard.cancelled.load(Ordering::Acquire) {
                    break;
                }
            }
        });
        guard
    }
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Ok(mut permit) = self.permit.lock() {
            permit.take();
        }
    }
    fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(io::ErrorKind::PermissionDenied.into())
        } else {
            Ok(())
        }
    }
    fn touch(&self) -> io::Result<()> {
        self.check()?;
        *self.last_poll.lock().map_err(|_| io::ErrorKind::Other)? = Instant::now();
        self.check()
    }
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
    let guard = StreamGuard::new(state.playback.clone(), lease.clone(), _permit);
    let work = StreamState {
        service: state.playback,
        lease,
        file: Arc::new(file),
        offset: start,
        remaining: length,
        guard,
    };
    let chunks = futures_util::stream::unfold(work, |mut s| async move {
        if s.remaining == 0 {
            return None;
        }
        let result = async {
            s.guard.touch()?;
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
            s.guard.check()?;
            Ok::<_, io::Error>(bytes)
        }
        .await;
        match result {
            Ok(bytes) => {
                s.offset += bytes.len() as u64;
                s.remaining -= bytes.len() as u64;
                if s.remaining == 0 {
                    s.guard.cancel();
                }
                Some((Ok(Bytes::from(bytes)), s))
            }
            Err(e) => {
                s.remaining = 0;
                s.guard.cancel();
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

    #[tokio::test]
    async fn four_real_stalled_jobs_timeout_without_releasing_worker_slots() {
        let service = PlaybackService::default();
        let (started_tx, mut started_rx) = tokio::sync::mpsc::channel(4);
        let mut releases = Vec::new();
        let mut jobs = Vec::new();
        for _ in 0..4 {
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            releases.push(release_tx);
            let started = started_tx.clone();
            let s = service.clone();
            jobs.push(tokio::spawn(async move {
                io_job(&s, move || {
                    started.blocking_send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(15)).unwrap();
                    Ok(())
                })
                .await
            }));
        }
        for _ in 0..4 {
            tokio::time::timeout(Duration::from_secs(2), started_rx.recv())
                .await
                .unwrap()
                .unwrap();
        }
        assert_eq!(service.io.available_permits(), 0);
        assert_eq!(
            io_job(&service, || Ok(())).await.unwrap_err().1,
            "MEDIA_BUSY"
        );
        for job in jobs {
            let error = job.await.unwrap().unwrap_err();
            assert_eq!(error.0, StatusCode::GATEWAY_TIMEOUT);
            assert_eq!(error.1, "MEDIA_TIMEOUT");
        }
        // Callers timed out, but the underlying workers are still running.
        assert_eq!(service.io.available_permits(), 0);
        assert_eq!(
            io_job(&service, || Ok(())).await.unwrap_err().1,
            "MEDIA_BUSY"
        );
        for release in releases {
            release.send(()).unwrap();
        }
        tokio::time::timeout(Duration::from_secs(2), async {
            while service.io.available_permits() != 4 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(io_job(&service, || Ok(())).await.is_ok());
    }
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
    #[test]
    fn subtitle_index_is_adjacent_bounded_and_never_follows_links() {
        let temp=tempfile::tempdir().unwrap(); let root=temp.path().canonicalize().unwrap(); let video=root.join("Film.mkv");
        std::fs::write(&video,b"video").unwrap(); std::fs::write(root.join("Film.en.srt"),b"1\n00:00:00,000 --> 00:00:01,000\nHi").unwrap();
        std::fs::write(root.join("Other.en.srt"),b"wrong").unwrap(); std::fs::write(root.join("Film.fr.srt"),vec![b'x';MAX_SUBTITLE_BYTES as usize+1]).unwrap();
        std::os::unix::fs::symlink("Film.en.srt",root.join("Film.de.srt")).unwrap();
        let got=subtitle_files(&root,&video).unwrap(); assert_eq!(got.len(),1); assert_eq!(got[0].0,"Film.en.srt"); assert_eq!(language(&got[0].0,"Film"),Some("en".into()));
    }
}
