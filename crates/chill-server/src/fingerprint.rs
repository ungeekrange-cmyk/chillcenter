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
    os::unix::fs::{FileExt, MetadataExt},
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
static SCAN_ACTIVE: AtomicBool = AtomicBool::new(false);
static SCAN_PENDING: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    static CONTENT_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn worker() -> Arc<Semaphore> {
    WORKER.get_or_init(|| Arc::new(Semaphore::new(1))).clone()
}

#[derive(Debug)]
struct CapturedFingerprint {
    evidence: FingerprintEvidence,
    file_version_token: String,
}

fn file_version_token(metadata: &std::fs::Metadata) -> String {
    let mut hash = Sha256::new();
    hash.update(b"chill-file-version-v1\0");
    hash.update(metadata.len().to_be_bytes());
    hash.update(metadata.dev().to_be_bytes());
    hash.update(metadata.ino().to_be_bytes());
    hash.update(metadata.mtime().to_be_bytes());
    hash.update(metadata.mtime_nsec().to_be_bytes());
    hash.update(metadata.ctime().to_be_bytes());
    hash.update(metadata.ctime_nsec().to_be_bytes());
    format!("{:x}", hash.finalize())
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
    let remaining = version
        .size
        .checked_sub(offset)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let wanted = bounded_read_length(remaining, length);
    wait_to_read(wanted, paused, cancelled)?;
    if FileVersion::from_metadata(&file.metadata()?)? != *version {
        return Err(io::ErrorKind::InvalidData.into());
    }
    #[cfg(test)]
    CONTENT_READS.with(|reads| reads.set(reads.get() + 1));
    let mut bytes = vec![0; wanted];
    let count = file.read_at(&mut bytes, offset)?;
    if count != wanted {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    bytes.truncate(count);
    Ok(bytes)
}

fn bounded_read_length(remaining: u64, limit: usize) -> usize {
    remaining.min(limit as u64) as usize
}

fn partial_hash(
    file: &File,
    version: &FileVersion,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<String> {
    let span = version.size.min(CHUNK_BYTES as u64);
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

fn capture_opened(
    file: File,
    version: FileVersion,
    file_version_token: String,
    kind: FingerprintKind,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<CapturedFingerprint> {
    if version.size == 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let partial_sha256 = partial_hash(&file, &version, paused, cancelled)?;
    let full_sha256 = if kind == FingerprintKind::Full {
        let mut hash = Sha256::new();
        let mut offset = 0;
        while offset < version.size {
            let bytes = read_chunk(&file, offset, CHUNK_BYTES, &version, paused, cancelled)?;
            offset += bytes.len() as u64;
            hash.update(bytes);
        }
        Some(format!("{:x}", hash.finalize()))
    } else {
        None
    };
    let final_metadata = file.metadata()?;
    if cancelled()
        || FileVersion::from_metadata(&final_metadata)? != version
        || crate::fingerprint::file_version_token(&final_metadata) != file_version_token
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(CapturedFingerprint {
        evidence: FingerprintEvidence {
            byte_size: version.size,
            partial_sha256,
            full_sha256,
        },
        file_version_token,
    })
}

fn open_for_capture(
    root: &Path,
    path: &Path,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<(File, FileVersion, String)> {
    wait_to_read(0, paused, cancelled)?;
    let (file, version) = open_indexed(root, path)?;
    let metadata = file.metadata()?;
    if FileVersion::from_metadata(&metadata)? != version {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let token = file_version_token(&metadata);
    Ok((file, version, token))
}

fn open_if_changed(
    root: &Path,
    path: &Path,
    observed_token: Option<&str>,
    evidence_current: bool,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<Option<(File, FileVersion, String)>> {
    let opened = open_for_capture(root, path, paused, cancelled)?;
    if evidence_current && observed_token == Some(opened.2.as_str()) {
        Ok(None)
    } else {
        Ok(Some(opened))
    }
}

fn capture(
    root: &Path,
    path: &Path,
    kind: FingerprintKind,
    paused: &dyn Fn() -> bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<CapturedFingerprint> {
    let (file, version, token) = open_for_capture(root, path, paused, cancelled)?;
    capture_opened(file, version, token, kind, paused, cancelled)
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
            let target = target.clone();
            move |store| store.mark_fingerprint_target_queued(&target, kind)
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
        let Some(root) = settings.root_for_path(&target.path) else {
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
            Ok(Ok(captured)) => {
                if state
                    .database
                    .call(move |store| {
                        store.save_fingerprint_versioned(
                            &target,
                            &captured.evidence,
                            &captured.file_version_token,
                        )
                    })
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

async fn scan_worker_permit() -> Option<tokio::sync::OwnedSemaphorePermit> {
    loop {
        while QUEUED.load(Ordering::Acquire) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let permit = worker().acquire_owned().await.ok()?;
        if QUEUED.load(Ordering::Acquire) == 0 {
            return Some(permit);
        }
        drop(permit);
        tokio::task::yield_now().await;
    }
}

fn root_revision_changed(state: &AppState, revision: u64) -> bool {
    state
        .library
        .lock()
        .map_or(true, |current| current.revision != revision)
}

async fn process_scan_target(state: &AppState, target: FingerprintTarget) -> bool {
    let Some(_permit) = scan_worker_permit().await else {
        return false;
    };
    let Some(settings) = state.library.lock().ok().map(|value| value.clone()) else {
        return false;
    };
    let Some(root) = settings.root_for_path(&target.path) else {
        return false;
    };
    let root_revision = settings.revision;
    if state
        .database
        .call({
            let target = target.clone();
            move |store| store.mark_fingerprint_target_queued(&target, FingerprintKind::Partial)
        })
        .await
        .is_err()
    {
        return true;
    }
    let media = target.media_id.clone();
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
        return true;
    }
    let playback = state.playback.clone();
    let library = state.library.clone();
    let path = target.path.clone();
    let observed_token = target.observed_file_version_token.clone();
    let observed_for_restore = observed_token.clone();
    let evidence_current = target.evidence_current;
    let opened = tokio::task::spawn_blocking(move || {
        open_if_changed(
            &root,
            &path,
            observed_token.as_deref(),
            evidence_current,
            &|| playback.fingerprint_should_pause(),
            &|| {
                library
                    .lock()
                    .map_or(true, |current| current.revision != root_revision)
            },
        )
    })
    .await;

    let opened = match opened {
        Ok(Ok(None)) => {
            let Some(token) = observed_for_restore else {
                let media = target.media_id.clone();
                let _ = state
                    .database
                    .call(move |store| store.mark_fingerprint_failed(&media))
                    .await;
                return true;
            };
            let restored = state
                .database
                .call(move |store| store.mark_fingerprint_unchanged(&target, &token))
                .await;
            if restored.is_err() {
                let media = media.clone();
                let _ = state
                    .database
                    .call(move |store| store.mark_fingerprint_failed(&media))
                    .await;
                return false;
            }
            return true;
        }
        Ok(Ok(Some(opened))) => opened,
        _ => {
            let root_changed = root_revision_changed(state, root_revision);
            let media = target.media_id.clone();
            let _ = state
                .database
                .call(move |store| store.mark_fingerprint_failed(&media))
                .await;
            if root_changed {
                return false;
            }
            return true;
        }
    };
    let media = target.media_id.clone();
    let playback = state.playback.clone();
    let library = state.library.clone();
    let captured = tokio::task::spawn_blocking(move || {
        capture_opened(
            opened.0,
            opened.1,
            opened.2,
            FingerprintKind::Partial,
            &|| playback.fingerprint_should_pause(),
            &|| {
                library
                    .lock()
                    .map_or(true, |current| current.revision != root_revision)
            },
        )
    })
    .await;
    if root_revision_changed(state, root_revision) {
        let media = target.media_id.clone();
        let _ = state
            .database
            .call(move |store| store.mark_fingerprint_failed(&media))
            .await;
        return false;
    }
    match captured {
        Ok(Ok(captured)) => {
            if state
                .database
                .call(move |store| {
                    store.save_fingerprint_versioned(
                        &target,
                        &captured.evidence,
                        &captured.file_version_token,
                    )
                })
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
    true
}

async fn run_scan_pump(state: AppState) {
    let mut cursor = String::new();
    loop {
        let after = cursor.clone();
        let targets = match state
            .database
            .call(move |store| store.fingerprint_scan_targets_after(&after, SCAN_BATCH))
            .await
        {
            Ok(targets) => targets,
            Err(_) => return,
        };
        if targets.is_empty() {
            return;
        }
        for target in targets {
            cursor.clone_from(&target.media_id);
            if !process_scan_target(&state, target).await {
                return;
            }
        }
    }
}

pub(crate) async fn enqueue_scan_candidates(state: AppState) {
    SCAN_PENDING.store(true, Ordering::Release);
    if SCAN_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    tokio::spawn(async move {
        loop {
            SCAN_PENDING.store(false, Ordering::Release);
            run_scan_pump(state.clone()).await;
            SCAN_ACTIVE.store(false, Ordering::Release);
            if !SCAN_PENDING.swap(false, Ordering::AcqRel)
                || SCAN_ACTIVE
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
            {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;
    #[test]
    fn lengths_are_bounded_before_conversion_for_armv7_large_files() {
        assert_eq!((1_u64 << 32) as u32, 0, "reproduces legacy ARM32 narrowing");
        for size in [1_u64 << 32, (1_u64 << 32) + 17, u64::MAX] {
            assert_eq!(bounded_read_length(size, CHUNK_BYTES), CHUNK_BYTES);
            assert_eq!(size.min(CHUNK_BYTES as u64), CHUNK_BYTES as u64);
        }
        assert_eq!(bounded_read_length(17, CHUNK_BYTES), 17);
    }

    fn state(temp: &tempfile::TempDir, root: &Path) -> AppState {
        let config = crate::config::ServerConfig {
            data_dir: temp.path().join("data"),
            video_root: Some(root.to_owned()),
            ..Default::default()
        };
        std::fs::create_dir(&config.data_dir).unwrap();
        let certificates = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        AppState {
            tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
            playback: Default::default(),
            scan: Arc::new(std::sync::Mutex::new(
                crate::catalog_api::ScanStatus::default(),
            )),
            database: crate::api::DatabaseActor::open(config.data_dir.join("catalog.sqlite"))
                .unwrap(),
            library: Arc::new(std::sync::Mutex::new(
                crate::config::LibrarySettings::load(&config).unwrap(),
            )),
            config: Arc::new(config),
            certificate: Arc::new(std::sync::Mutex::new(certificates.schedule)),
        }
    }

    async fn snapshot(state: &AppState, root: &Path, paths: Vec<std::path::PathBuf>) {
        let root = root.to_owned();
        state
            .database
            .call(move |store| {
                let profile = store.create_profile("fingerprint fixture")?;
                store.snapshot(
                    paths
                        .iter()
                        .map(|path| chill_catalog::scanner::parse_file(&root, path))
                        .collect(),
                )?;
                store.catalog(&profile.id)?;
                Ok(())
            })
            .await
            .unwrap();
    }

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
        let checks = AtomicUsize::new(0);
        let result = capture(&root, &path, FingerprintKind::Full, &|| false, &|| {
            if checks.fetch_add(1, Ordering::AcqRel) == 2 {
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

    #[tokio::test]
    async fn scan_pump_advances_past_failed_row_and_first_page() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("videos");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let mut paths = Vec::new();
        for index in 0..20 {
            let path = root.join(format!("video-{index:02}.mp4"));
            std::fs::write(&path, [index as u8 + 1]).unwrap();
            paths.push(path);
        }
        let state = state(&temp, &root);
        snapshot(&state, &root, paths).await;
        let targets = state
            .database
            .call(|store| store.fingerprint_scan_targets_after("", 32))
            .await
            .unwrap();
        assert_eq!(targets.len(), 20);
        let failed_id = targets[0].media_id.clone();
        std::fs::remove_file(&targets[0].path).unwrap();

        run_scan_pump(state.clone()).await;

        for target in targets {
            let media = target.media_id.clone();
            let status = state
                .database
                .call(move |store| store.fingerprint_status(&media))
                .await
                .unwrap();
            if target.media_id == failed_id {
                assert_eq!(status.state, "failed");
            } else {
                assert_eq!(status.state, "complete");
                assert_eq!(status.strength.as_deref(), Some("partial"));
            }
        }
    }

    #[tokio::test]
    async fn changed_file_with_same_size_and_partial_invalidates_full_proof() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("videos");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let path = root.join("same.mp4");
        let original_bytes = vec![7_u8; 512 * 1024];
        std::fs::write(&path, &original_bytes).unwrap();
        let state = state(&temp, &root);
        snapshot(&state, &root, vec![path.clone()]).await;
        let target = state
            .database
            .call(|store| {
                store
                    .fingerprint_scan_targets_after("", 1)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| "missing target".to_owned())
            })
            .await
            .unwrap();
        let captured = capture(&root, &path, FingerprintKind::Full, &|| false, &|| false).unwrap();
        let original_partial = captured.evidence.partial_sha256.clone();
        let saved_target = target.clone();
        state
            .database
            .call(move |store| {
                store.save_fingerprint_versioned(
                    &saved_target,
                    &captured.evidence,
                    &captured.file_version_token,
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let current = state
            .database
            .call({
                let media = target.media_id.clone();
                move |store| store.fingerprint_target(&media)
            })
            .await
            .unwrap();
        CONTENT_READS.with(|reads| reads.set(0));
        assert!(
            open_if_changed(
                &root,
                &path,
                current.observed_file_version_token.as_deref(),
                current.evidence_current,
                &|| false,
                &|| false,
            )
            .unwrap()
            .is_none(),
            "an unchanged descriptor must complete after stat without a content read"
        );
        CONTENT_READS.with(|reads| assert_eq!(reads.get(), 0));

        let mut replacement_bytes = original_bytes;
        replacement_bytes[100_000] ^= 0xff;
        let replacement = root.join("replacement.tmp");
        std::fs::write(&replacement, replacement_bytes).unwrap();
        std::fs::rename(&replacement, &path).unwrap();
        let replacement_partial =
            capture(&root, &path, FingerprintKind::Partial, &|| false, &|| false).unwrap();
        assert_eq!(
            replacement_partial.evidence.partial_sha256, original_partial,
            "fixture change deliberately falls outside all partial samples"
        );

        run_scan_pump(state.clone()).await;

        let media = target.media_id;
        let status = state
            .database
            .call(move |store| store.fingerprint_status(&media))
            .await
            .unwrap();
        assert_eq!(status.state, "complete");
        assert_eq!(status.strength.as_deref(), Some("partial"));
    }
}
