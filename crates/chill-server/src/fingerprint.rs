//! Bounded, playback-aware media fingerprint worker.
use crate::{
    api::{ApiError, AppState},
    media_stream::{FileVersion, open_indexed},
};
use chill_domain::fingerprint::{FingerprintEvidence, FingerprintKind, FingerprintTarget};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io,
    os::unix::fs::FileExt,
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Semaphore;

const RATE_BYTES_PER_SECOND: u64 = 1024 * 1024;
const CHUNK_BYTES: usize = 64 * 1024;
const MAX_QUEUED_JOBS: usize = 16;
const SCAN_BATCH: usize = 16;

static WORKER: OnceLock<Arc<Semaphore>> = OnceLock::new();
static QUEUED: AtomicUsize = AtomicUsize::new(0);

fn worker() -> Arc<Semaphore> {
    WORKER.get_or_init(|| Arc::new(Semaphore::new(1))).clone()
}

struct QueueSlot;
impl QueueSlot {
    fn reserve() -> Result<Self, ApiError> {
        QUEUED
            .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < MAX_QUEUED_JOBS).then_some(count + 1)
            })
            .map_err(|_| {
                ApiError(
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    "FINGERPRINT_BUSY",
                )
            })?;
        Ok(Self)
    }
}
impl Drop for QueueSlot {
    fn drop(&mut self) {
        QUEUED.fetch_sub(1, Ordering::AcqRel);
    }
}

fn wait_to_read(
    bytes: usize,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<()> {
    loop {
        while paused() {
            if cancelled() {
                return Err(io::ErrorKind::Interrupted.into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        if cancelled() {
            return Err(io::ErrorKind::Interrupted.into());
        }
        std::thread::sleep(Duration::from_secs_f64(
            bytes as f64 / RATE_BYTES_PER_SECOND as f64,
        ));
        if !paused() {
            return if cancelled() {
                Err(io::ErrorKind::Interrupted.into())
            } else {
                Ok(())
            };
        }
    }
}

fn read_chunk(
    file: &File,
    offset: u64,
    length: usize,
    version: &FileVersion,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<Vec<u8>> {
    let wanted = length.min((version.size - offset) as usize);
    wait_to_read(wanted, paused, cancelled)?;
    if FileVersion::from_metadata(&file.metadata()?)? != *version {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut bytes = vec![0; wanted];
    let count = file.read_at(&mut bytes, offset)?;
    if count != wanted {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    bytes.truncate(count);
    Ok(bytes)
}

fn partial_hash(
    file: &File,
    version: &FileVersion,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<String> {
    let span = CHUNK_BYTES.min(version.size as usize) as u64;
    let mut offsets = vec![
        0,
        version.size.saturating_sub(span) / 2,
        version.size.saturating_sub(span),
    ];
    offsets.sort_unstable();
    offsets.dedup();
    let mut hash = Sha256::new();
    hash.update(b"chill-partial-v1\0");
    hash.update(version.size.to_be_bytes());
    for offset in offsets {
        hash.update(offset.to_be_bytes());
        hash.update(read_chunk(
            file,
            offset,
            span as usize,
            version,
            paused,
            cancelled,
        )?);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn capture(
    root: &Path,
    path: &Path,
    kind: FingerprintKind,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<FingerprintEvidence> {
    let (file, version) = open_indexed(root, path)?;
    if version.size == 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let partial_sha256 = partial_hash(&file, &version, paused, cancelled)?;
    let full_sha256 = if kind == FingerprintKind::Full {
        let mut hash = Sha256::new();
        let mut offset = 0;
        while offset < version.size {
            let bytes = read_chunk(
                &file,
                offset,
                CHUNK_BYTES,
                &version,
                paused,
                cancelled,
            )?;
            offset += bytes.len() as u64;
            hash.update(bytes);
        }
        Some(format!("{:x}", hash.finalize()))
    } else {
        None
    };
    if cancelled() || FileVersion::from_metadata(&file.metadata()?)? != version {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(FingerprintEvidence {
        byte_size: version.size,
        partial_sha256,
        full_sha256,
    })
}

async fn enqueue_target(
    state: AppState,
    target: FingerprintTarget,
    kind: FingerprintKind,
) -> Result<(), ApiError> {
    let slot = QueueSlot::reserve()?;
    let media = target.media_id.clone();
    state
        .database
        .call({
            let media = media.clone();
            move |store| store.mark_fingerprint_queued(&media, kind)
        })
        .await?;
    tokio::spawn(async move {
        let _slot = slot;
        let Ok(_permit) = worker().acquire_owned().await else {
            return;
        };
        if state
            .database
            .call({
                let media = media.clone();
                let revision = target.media_revision;
                move |store| store.mark_fingerprint_running(&media, revision)
            })
            .await
            .is_err()
        {
            let _ = state
                .database
                .call(move |store| store.mark_fingerprint_failed(&media))
                .await;
            return;
        }
        let settings = state.library.lock().ok().map(|value| value.clone());
        let Some(settings) = settings else {
            let _ = state
                .database
                .call(move |store| store.mark_fingerprint_failed(&media))
                .await;
            return;
        };
        let Some(root) = settings.video_root.clone() else {
            let _ = state
                .database
                .call(move |store| store.mark_fingerprint_failed(&media))
                .await;
            return;
        };
        let playback = state.playback.clone();
        let library = state.library.clone();
        let path = target.path.clone();
        let root_revision = settings.revision;
        let result = tokio::task::spawn_blocking(move || {
            capture(
                &root,
                &path,
                kind,
                &|| playback.fingerprint_should_pause(),
                &|| {
                    library
                        .lock()
                        .map_or(true, |current| current.revision != root_revision)
                },
            )
        })
        .await;
        match result {
            Ok(Ok(evidence)) => {
                if state
                    .database
                    .call(move |store| store.save_fingerprint(&target, &evidence))
                    .await
                    .is_err()
                {
                    let _ = state
                        .database
                        .call(move |store| store.mark_fingerprint_failed(&media))
                        .await;
                }
            }
            _ => {
                let _ = state
                    .database
                    .call(move |store| store.mark_fingerprint_failed(&media))
                    .await;
            }
        }
    });
    Ok(())
}

pub(crate) async fn enqueue_full(state: AppState, media_id: String) -> Result<(), ApiError> {
    let lookup = media_id.clone();
    let target = state
        .database
        .call(move |store| store.fingerprint_target(&lookup))
        .await?;
    enqueue_target(state, target, FingerprintKind::Full).await
}

pub(crate) async fn enqueue_scan_candidates(state: AppState) {
    let targets = match state
        .database
        .call(|store| store.partial_fingerprint_targets(SCAN_BATCH))
        .await
    {
        Ok(targets) => targets,
        Err(_) => return,
    };
    for target in targets {
        if enqueue_target(state.clone(), target, FingerprintKind::Partial)
            .await
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    #[test]
    fn partial_is_rate_limited_and_suspends_before_reading() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let path = root.join("video.bin");
        std::fs::write(&path, vec![7_u8; 128 * 1024]).unwrap();
        let paused = AtomicBool::new(true);
        let release = &paused;
        std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(80));
                release.store(false, Ordering::Release);
            });
            let started = Instant::now();
            capture(
                &root,
                &path,
                FingerprintKind::Partial,
                &|| paused.load(Ordering::Acquire),
                &|| false,
            )
            .unwrap();
            assert!(started.elapsed() >= Duration::from_millis(240));
        });
    }

    #[test]
    fn descriptor_modification_cancels_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let path = root.join("video.bin");
        std::fs::write(&path, vec![3_u8; 256 * 1024]).unwrap();
        let changed = AtomicBool::new(false);
        let result = capture(&root, &path, FingerprintKind::Full, &|| false, &|| {
            if !changed.swap(true, Ordering::AcqRel) {
                std::fs::write(&path, vec![4_u8; 256 * 1024]).unwrap();
            }
            false
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn root_change_cancels_before_any_evidence_is_returned() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let path = root.join("video.bin");
        std::fs::write(&path, vec![9_u8; 128 * 1024]).unwrap();
        assert_eq!(
            capture(&root, &path, FingerprintKind::Partial, &|| false, &|| true)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
    }
}
