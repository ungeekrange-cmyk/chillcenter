//! Private provider seam; production always uses the fixed-host TMDB client.
use super::{AppState, JobStatus, TmdbJobService, bump, provider};
use chill_catalog::{
    metadata::{Candidate, CandidateSearch, ProviderFailure, Tmdb},
    model::Media,
};
use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc, time::Duration};

type Request<'a, T> = Pin<Box<dyn Future<Output = Result<T, ProviderFailure>> + Send + 'a>>;
pub(super) trait Provider: Send + Sync {
    fn candidates(&self, media: Media) -> Request<'_, CandidateSearch>;
    fn enrich(&self, media: Media, candidate: Candidate, cache: PathBuf) -> Request<'_, Media>;
}
impl Provider for Tmdb {
    fn candidates(&self, media: Media) -> Request<'_, CandidateSearch> {
        Box::pin(async move { self.candidates_for_job_with_context(&media, "").await })
    }
    fn enrich(&self, media: Media, candidate: Candidate, cache: PathBuf) -> Request<'_, Media> {
        Box::pin(async move {
            self.enrich_for_job(media, candidate.id, &candidate.kind, &cache)
                .await
        })
    }
}
#[derive(Clone, Copy)]
pub(super) struct Policy {
    rate: Duration,
    backoff: Duration,
    poll: Duration,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            rate: Duration::from_secs(1),
            backoff: Duration::from_secs(1),
            poll: Duration::from_millis(100),
        }
    }
}
fn cancelled(service: &TmdbJobService, id: &str) -> bool {
    service
        .status(id)
        .is_none_or(|job| job.cancelled || matches!(job.status, JobStatus::Cancelled))
}
async fn ready(
    service: &TmdbJobService,
    id: &str,
    playback: &(dyn Fn() -> bool + Send + Sync),
    policy: Policy,
) -> bool {
    loop {
        if cancelled(service, id) {
            return false;
        }
        if !playback() {
            if service
                .status(id)
                .is_some_and(|j| !matches!(j.status, JobStatus::Running))
            {
                bump(service, id, |j| j.status = JobStatus::Running);
            }
            return true;
        }
        if service
            .status(id)
            .is_some_and(|j| !matches!(j.status, JobStatus::PausedPlayback))
        {
            bump(service, id, |j| j.status = JobStatus::PausedPlayback);
        }
        tokio::time::sleep(policy.poll).await;
    }
}
async fn request<'a, T>(
    service: &TmdbJobService,
    id: &str,
    playback: &(dyn Fn() -> bool + Send + Sync),
    policy: Policy,
    mut call: impl FnMut() -> Request<'a, T>,
) -> Option<Result<T, ProviderFailure>> {
    for attempt in 0..3 {
        // The serialized queue spaces every search/detail request, including
        // requests belonging to separate jobs. Backoff is additional to rate.
        tokio::time::sleep(policy.rate).await;
        if !ready(service, id, playback, policy).await {
            return None;
        }
        let result = call().await;
        if cancelled(service, id) {
            return Some(result);
        }
        match result {
            Err(ref error) if error.retryable && attempt < 2 => {
                tokio::time::sleep(policy.backoff * (1 << attempt)).await
            }
            _ => return Some(result),
        }
    }
    unreachable!("three attempts return a result")
}
fn preserve_known(source: &Media, downloaded: &mut Media) {
    downloaded.title = source.title.clone();
    downloaded.series = source.series.clone();
    downloaded.season = source.season;
    downloaded.episode = source.episode;
    if source.year.is_some() {
        downloaded.year = source.year;
    }
    if !source.overview.is_empty() {
        downloaded.overview = source.overview.clone();
    }
    if !source.genres.is_empty() {
        downloaded.genres = source.genres.clone();
    }
    if !source.cast.is_empty() {
        downloaded.cast = source.cast.clone();
    }
    if !source.directors.is_empty() {
        downloaded.directors = source.directors.clone();
    }
    if source.runtime_minutes.is_some() {
        downloaded.runtime_minutes = source.runtime_minutes;
    }
    if !source.series_overview.is_empty() {
        downloaded.series_overview = source.series_overview.clone();
    }
}
pub(super) async fn run(
    state: AppState,
    service: Arc<TmdbJobService>,
    id: String,
    injected: Option<Arc<dyn Provider>>,
    playback: Arc<dyn Fn() -> bool + Send + Sync>,
    policy: Policy,
    task: super::JobTask,
) {
    let _task = task;
    // Only one provider worker can make requests/write metadata at a time.
    let _owner = service.worker_gate.lock().await;
    if !ready(&service, &id, playback.as_ref(), policy).await {
        return;
    }
    let Some(job) = service.status(&id) else {
        return;
    };
    if job.total == 0 {
        bump(&service, &id, |j| j.status = JobStatus::Completed);
        return;
    }
    let provider: Arc<dyn Provider> = match injected {
        Some(provider) => provider,
        None => match provider(&state) {
            Ok(provider) => Arc::new(provider),
            Err(_) => {
                bump(&service, &id, |j| {
                    j.failed = j.total;
                    j.processed = j.total;
                    j.status = JobStatus::Failed;
                });
                return;
            }
        },
    };
    for media_id in job.media_ids {
        if !ready(&service, &id, playback.as_ref(), policy).await {
            return;
        }
        let profile = job.profile_id.clone();
        let item_id = media_id.clone();
        let snapshot = state
            .database
            .call(move |s| {
                let revision = s.media_revision(&item_id)?;
                s.provider_snapshot(&profile, &item_id, revision)
            })
            .await;
        let snapshot = match snapshot {
            Ok(snapshot) => snapshot,
            Err(error) => {
                bump(&service, &id, |j| {
                    j.processed += 1;
                    if error.0 == axum::http::StatusCode::NOT_FOUND {
                        j.skipped += 1;
                    } else {
                        j.failed += 1;
                        j.retryable += usize::from(
                            error.0.is_server_error()
                                || error.0 == axum::http::StatusCode::REQUEST_TIMEOUT,
                        );
                    }
                });
                continue;
            }
        };
        if snapshot.media.tmdb_id.is_some()
            || snapshot.media.poster_custom
            || snapshot.media.series_manual
            || !snapshot.media.available
        {
            bump(&service, &id, |j| {
                j.processed += 1;
                j.skipped += 1;
            });
            continue;
        }
        let Some(candidates) = request(&service, &id, playback.as_ref(), policy, || {
            provider.candidates(snapshot.media.clone())
        })
        .await
        else {
            return;
        };
        let candidates = match candidates {
            Ok(candidates) => candidates,
            Err(error) => {
                bump(&service, &id, |j| {
                    j.processed += 1;
                    j.failed += 1;
                    j.retryable += usize::from(error.retryable);
                });
                continue;
            }
        };
        let Some(candidate) =
            chill_catalog::metadata::exact_automatic_candidate(&snapshot.media, &candidates)
        else {
            bump(&service, &id, |j| {
                j.processed += 1;
                j.ambiguous += 1;
            });
            continue;
        };
        let Some(downloaded) = request(&service, &id, playback.as_ref(), policy, || {
            provider.enrich(
                snapshot.media.clone(),
                candidate.clone(),
                state.config.data_dir.join("posters"),
            )
        })
        .await
        else {
            return;
        };
        let mut downloaded = match downloaded {
            Ok(downloaded) => downloaded,
            Err(error) => {
                bump(&service, &id, |j| {
                    j.processed += 1;
                    j.failed += 1;
                    j.retryable += usize::from(error.retryable);
                });
                continue;
            }
        };
        let owned = downloaded
            .poster
            .clone()
            .filter(|path| snapshot.media.poster.as_ref() != Some(path));
        if !ready(&service, &id, playback.as_ref(), policy).await {
            if let Some(path) = owned {
                let _ = std::fs::remove_file(path);
            }
            return;
        }
        preserve_known(&snapshot.media, &mut downloaded);
        let profile = job.profile_id.clone();
        let guard_service = service.clone();
        let guard_id = id.clone();
        let result = state
            .database
            .call(move |s| {
                // Order cancellation acknowledgment against the actual CAS/write.
                let jobs = guard_service
                    .jobs
                    .lock()
                    .map_err(|_| "Traitement TMDB indisponible.")?;
                if jobs
                    .get(&guard_id)
                    .is_none_or(|job| job.cancelled || matches!(job.status, JobStatus::Cancelled))
                {
                    return Err("Traitement TMDB annulé.".into());
                }
                s.provider_apply_media(&profile, &media_id, &snapshot, downloaded, false)
            })
            .await;
        if result.is_err()
            && let Some(path) = owned
        {
            let _ = std::fs::remove_file(path);
        }
        bump(&service, &id, |j| {
            j.processed += 1;
            if result.is_ok() {
                j.matched += 1;
            } else {
                j.failed += 1;
            }
        });
    }
    bump(&service, &id, |j| {
        j.status = if j.failed == 0 {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::DatabaseActor,
        config::{LibrarySettings, ServerConfig},
    };
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use tokio::sync::{Notify, Semaphore};

    struct Fixture {
        _dir: tempfile::TempDir,
        state: AppState,
        profile: String,
        ids: Vec<String>,
    }
    fn fixture(count: usize) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().into(),
            ..Default::default()
        };
        let path = dir.path().join("catalog.sqlite");
        let mut store = chill_domain::Store::open(&path).unwrap();
        let items = (0..count).map(|i| serde_json::from_value(serde_json::json!({
            "id":format!("film{i}.mkv"),"path":format!("/synthetic/film{i}.mkv"),"title":"Alien","category":"film","year":1979,"series":null,"season":null,"episode":null,"genres":[],"overview":"","poster":null,"saga":null,"watched":false,"available":true,"added_at":0,"tmdb_id":null
        })).unwrap()).collect();
        store.snapshot(items).unwrap();
        let profile = store.create_profile("Synthetic worker test").unwrap().id;
        let ids = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        drop(store);
        let cert = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
        let state = AppState {
            tmdb_jobs: Arc::new(TmdbJobService::initialize(dir.path()).unwrap()),
            playback: Default::default(),
            database: DatabaseActor::open(path).unwrap(),
            library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
            certificate: Arc::new(Mutex::new(cert.schedule)),
            scan: Arc::new(Mutex::new(Default::default())),
            config: Arc::new(config),
        };
        Fixture {
            _dir: dir,
            state,
            profile,
            ids,
        }
    }
    struct Fake {
        calls: Mutex<Vec<(&'static str, String, std::time::Instant)>>,
        search_failures: AtomicUsize,
        detail_failures: AtomicUsize,
        permanent: bool,
        ambiguous: bool,
        pause_after_search: Option<Arc<AtomicBool>>,
        detail_entered: Notify,
        detail_gate: Option<Arc<Semaphore>>,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                search_failures: AtomicUsize::new(0),
                detail_failures: AtomicUsize::new(0),
                permanent: false,
                ambiguous: false,
                pause_after_search: None,
                detail_entered: Notify::new(),
                detail_gate: None,
            }
        }
    }
    impl Fake {
        fn record(&self, phase: &'static str, media: &Media) {
            self.calls
                .lock()
                .unwrap()
                .push((phase, media.id.clone(), std::time::Instant::now()));
        }
        fn fail(&self, count: &AtomicUsize) -> bool {
            count
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok()
        }
        fn failure(&self) -> ProviderFailure {
            let mut error = ProviderFailure::from("Synthetic error without provider secret");
            error.retryable = !self.permanent;
            error
        }
    }
    impl Provider for Fake {
        fn candidates(&self, media: Media) -> Request<'_, CandidateSearch> {
            Box::pin(async move {
                self.record("search", &media);
                if self.fail(&self.search_failures) {
                    return Err(self.failure());
                }
                if let Some(pause) = &self.pause_after_search {
                    pause.store(true, Ordering::SeqCst);
                }
                let candidate = Candidate {
                    id: 348,
                    title: "Alien".into(),
                    year: Some(1979),
                    overview: String::new(),
                    poster: Some("https://image.tmdb.org/t/p/w342/fake.jpg".into()),
                    kind: "movie".into(),
                };
                Ok(CandidateSearch {
                    candidates: if self.ambiguous {
                        vec![candidate.clone(), candidate]
                    } else {
                        vec![candidate]
                    },
                    fallback_used: false,
                })
            })
        }
        fn enrich(
            &self,
            mut media: Media,
            candidate: Candidate,
            cache: PathBuf,
        ) -> Request<'_, Media> {
            Box::pin(async move {
                self.record("detail", &media);
                self.detail_entered.notify_one();
                if let Some(gate) = &self.detail_gate {
                    gate.acquire().await.unwrap().forget();
                }
                if self.fail(&self.detail_failures) {
                    return Err(self.failure());
                }
                media.tmdb_id = Some(candidate.id);
                media.title = "Provider changed case".into();
                media.overview = "Provider synopsis".into();
                media.genres = vec!["Science fiction".into()];
                media.runtime_minutes = Some(117);
                media.cast = vec!["Synthetic actor".into()];
                media.saga = Some("Provider saga".into());
                media.poster = Some(
                    chill_catalog::poster::store(include_bytes!("../../../icons/icon.png"), &cache)
                        .unwrap(),
                );
                Ok(media)
            })
        }
    }
    fn policy() -> Policy {
        Policy {
            rate: Duration::from_millis(10),
            backoff: Duration::from_millis(15),
            poll: Duration::from_millis(5),
        }
    }
    fn launch(
        f: &Fixture,
        provider: Arc<Fake>,
        pause: Arc<AtomicBool>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let (job, task_guard) = f
            .state
            .tmdb_jobs
            .create(f.profile.clone(), f.ids.clone())
            .unwrap();
        let id = job.job_id;
        let state = f.state.clone();
        let worker_id = id.clone();
        let task = tokio::spawn(async move {
            run(
                state.clone(),
                state.tmdb_jobs.clone(),
                worker_id,
                Some(provider),
                Arc::new(move || pause.load(Ordering::SeqCst)),
                policy(),
                task_guard,
            )
            .await;
        });
        (id, task)
    }
    async fn item(f: &Fixture, index: usize) -> Media {
        let profile = f.profile.clone();
        let id = f.ids[index].clone();
        f.state
            .database
            .call(move |s| s.media(&profile, &id))
            .await
            .unwrap()
    }
    async fn finish(task: tokio::task::JoinHandle<()>) {
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
    }
    async fn wait_paused(service: &TmdbJobService, id: &str) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if service
                    .status(id)
                    .is_some_and(|j| matches!(j.status, JobStatus::PausedPlayback))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn spawned_worker_writes_metadata_artwork_counters_and_terminal_state() {
        let f = fixture(1);
        let provider = Arc::new(Fake::default());
        let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(false)));
        finish(task).await;
        let media = item(&f, 0).await;
        let job = f.state.tmdb_jobs.status(&id).unwrap();
        assert_eq!(media.tmdb_id, Some(348));
        assert_eq!(media.overview, "Provider synopsis");
        assert_eq!(media.runtime_minutes, Some(117));
        assert_eq!(media.title, "Alien");
        assert!(media.poster.unwrap().starts_with("/api/"));
        assert!(matches!(job.status, JobStatus::Completed));
        assert_eq!(
            (job.total, job.processed, job.matched, job.failed),
            (1, 1, 1, 0)
        );
        assert_eq!(
            provider
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|(phase, _, _)| *phase)
                .collect::<Vec<_>>(),
            vec!["search", "detail"]
        );
        let files = std::fs::read_dir(f.state.config.data_dir.join("posters"))
            .unwrap()
            .count();
        assert_eq!(files, 1);
        let recovered = TmdbJobService::initialize(&f.state.config.data_dir).unwrap();
        assert!(matches!(
            recovered.status(&id).unwrap().status,
            JobStatus::Completed
        ));
        let persisted =
            std::fs::read_to_string(f.state.config.data_dir.join("tmdb-jobs.json")).unwrap();
        assert!(!persisted.contains("http"));
        assert!(!persisted.contains("Provider synopsis"));
        assert!(!persisted.contains("credential"));
    }
    #[tokio::test]
    async fn ambiguity_is_no_write_and_not_a_retryable_provider_failure() {
        let f = fixture(1);
        let provider = Arc::new(Fake {
            ambiguous: true,
            ..Default::default()
        });
        let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(false)));
        finish(task).await;
        let job = f.state.tmdb_jobs.status(&id).unwrap();
        assert_eq!(
            (job.processed, job.ambiguous, job.failed, job.retryable),
            (1, 1, 0, 0)
        );
        assert!(item(&f, 0).await.tmdb_id.is_none());
        assert_eq!(provider.calls.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn empty_job_completes_without_loading_a_provider() {
        let f = fixture(0);
        let (job, task_guard) = f
            .state
            .tmdb_jobs
            .create(f.profile.clone(), Vec::new())
            .unwrap();
        run(
            f.state.clone(),
            f.state.tmdb_jobs.clone(),
            job.job_id.clone(),
            None,
            Arc::new(|| false),
            policy(),
            task_guard,
        )
        .await;
        let job = f.state.tmdb_jobs.status(&job.job_id).unwrap();
        assert_eq!((job.total, job.processed), (0, 0));
        assert!(matches!(job.status, JobStatus::Completed));
    }
    #[tokio::test]
    async fn duplicate_ids_and_missing_media_have_consistent_total_and_processed_counts() {
        let mut f = fixture(1);
        f.ids.push(f.ids[0].clone());
        f.ids.push(uuid::Uuid::new_v4().to_string());
        let provider = Arc::new(Fake::default());
        let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(false)));
        finish(task).await;
        let job = f.state.tmdb_jobs.status(&id).unwrap();
        assert_eq!(
            (
                job.total,
                job.processed,
                job.matched,
                job.skipped,
                job.failed
            ),
            (2, 2, 1, 1, 0)
        );
        assert_eq!(provider.calls.lock().unwrap().len(), 2);
    }
    #[tokio::test]
    async fn preexisting_manual_values_are_filled_around_and_custom_posters_are_skipped() {
        let f = fixture(2);
        let profile = f.profile.clone();
        let a = f.ids[0].clone();
        let b = f.ids[1].clone();
        f.state
            .database
            .call(move |s| {
                s.update_media_at_revision(
                    &profile,
                    &a,
                    1,
                    &chill_catalog::model::MediaPatch {
                        overview: Some("Manual synopsis".into()),
                        genres: Some(vec!["Manual genre".into()]),
                        saga: Some("Manual saga".into()),
                        ..Default::default()
                    },
                )?;
                let snap = s.provider_snapshot(&profile, &b, 1)?;
                let mut custom = snap.media.clone();
                custom.poster = Some("/synthetic/custom.png".into());
                custom.poster_custom = true;
                s.provider_apply_media(&profile, &b, &snap, custom, true)?;
                Ok(())
            })
            .await
            .unwrap();
        let (id, task) = launch(
            &f,
            Arc::new(Fake::default()),
            Arc::new(AtomicBool::new(false)),
        );
        finish(task).await;
        let updated = item(&f, 0).await;
        assert_eq!(updated.overview, "Manual synopsis");
        assert_eq!(updated.genres, ["Manual genre"]);
        assert_eq!(updated.saga.as_deref(), Some("Manual saga"));
        assert_eq!(updated.tmdb_id, Some(348));
        let custom = item(&f, 1).await;
        assert!(custom.poster_custom);
        assert!(custom.tmdb_id.is_none());
        let job = f.state.tmdb_jobs.status(&id).unwrap();
        assert_eq!((job.processed, job.matched, job.skipped), (2, 1, 1));
    }
    #[tokio::test]
    async fn concurrent_manual_change_wins_cas_and_downloaded_artwork_is_cleaned() {
        let f = fixture(1);
        let gate = Arc::new(Semaphore::new(0));
        let provider = Arc::new(Fake {
            detail_gate: Some(gate.clone()),
            ..Default::default()
        });
        let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(false)));
        provider.detail_entered.notified().await;
        let profile = f.profile.clone();
        let media = f.ids[0].clone();
        f.state
            .database
            .call(move |s| {
                s.update_media_at_revision(
                    &profile,
                    &media,
                    1,
                    &chill_catalog::model::MediaPatch {
                        title: Some("Concurrent correction".into()),
                        ..Default::default()
                    },
                )
                .map(|_| ())
            })
            .await
            .unwrap();
        gate.add_permits(1);
        finish(task).await;
        let media = item(&f, 0).await;
        assert_eq!(media.title, "Concurrent correction");
        assert!(media.tmdb_id.is_none());
        let job = f.state.tmdb_jobs.status(&id).unwrap();
        assert_eq!((job.processed, job.failed, job.retryable), (1, 1, 0));
        assert!(matches!(job.status, JobStatus::Failed));
        assert_eq!(
            std::fs::read_dir(f.state.config.data_dir.join("posters"))
                .unwrap()
                .count(),
            0
        );
    }
    #[tokio::test]
    async fn playback_between_search_and_detail_resumes_the_same_item() {
        let f = fixture(1);
        let paused = Arc::new(AtomicBool::new(false));
        let provider = Arc::new(Fake {
            pause_after_search: Some(paused.clone()),
            ..Default::default()
        });
        let (id, task) = launch(&f, provider.clone(), paused.clone());
        wait_paused(&f.state.tmdb_jobs, &id).await;
        assert_eq!(provider.calls.lock().unwrap().len(), 1);
        assert!(item(&f, 0).await.tmdb_id.is_none());
        paused.store(false, Ordering::SeqCst);
        finish(task).await;
        {
            let calls = provider.calls.lock().unwrap();
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0].1, calls[1].1);
        }
        assert_eq!(item(&f, 0).await.tmdb_id, Some(348));
    }
    #[tokio::test]
    async fn cancellation_while_paused_or_during_detail_never_writes_or_completes() {
        for in_detail in [false, true] {
            let f = fixture(1);
            let gate = Arc::new(Semaphore::new(0));
            let provider = Arc::new(Fake {
                detail_gate: in_detail.then(|| gate.clone()),
                ..Default::default()
            });
            let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(!in_detail)));
            if in_detail {
                provider.detail_entered.notified().await;
            } else {
                wait_paused(&f.state.tmdb_jobs, &id).await;
            }
            f.state.tmdb_jobs.cancel(&id).unwrap();
            gate.add_permits(1);
            finish(task).await;
            assert!(matches!(
                f.state.tmdb_jobs.status(&id).unwrap().status,
                JobStatus::Cancelled
            ));
            assert!(item(&f, 0).await.tmdb_id.is_none());
            if let Ok(files) = std::fs::read_dir(f.state.config.data_dir.join("posters")) {
                assert_eq!(files.count(), 0);
            }
        }
    }
    #[tokio::test]
    async fn transient_search_and_detail_retry_three_times_with_rate_and_backoff() {
        for search in [true, false] {
            let f = fixture(1);
            let provider = Arc::new(Fake {
                search_failures: AtomicUsize::new(if search { 2 } else { 0 }),
                detail_failures: AtomicUsize::new(if search { 0 } else { 2 }),
                ..Default::default()
            });
            let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(false)));
            finish(task).await;
            assert!(matches!(
                f.state.tmdb_jobs.status(&id).unwrap().status,
                JobStatus::Completed
            ));
            assert_eq!(item(&f, 0).await.tmdb_id, Some(348));
            let phase = if search { "search" } else { "detail" };
            let calls = provider.calls.lock().unwrap();
            let attempts = calls
                .iter()
                .filter(|(p, _, _)| *p == phase)
                .collect::<Vec<_>>();
            assert_eq!(attempts.len(), 3);
            assert!(
                attempts[1].2.duration_since(attempts[0].2) >= policy().rate + policy().backoff
            );
            assert!(
                attempts[2].2.duration_since(attempts[1].2) >= policy().rate + policy().backoff * 2
            );
        }
    }
    #[tokio::test]
    async fn exhausted_transient_and_permanent_failures_are_never_counted_as_ambiguity() {
        for permanent in [false, true] {
            let f = fixture(1);
            let provider = Arc::new(Fake {
                search_failures: AtomicUsize::new(20),
                permanent,
                ..Default::default()
            });
            let (id, task) = launch(&f, provider.clone(), Arc::new(AtomicBool::new(false)));
            finish(task).await;
            let job = f.state.tmdb_jobs.status(&id).unwrap();
            assert_eq!(
                (job.processed, job.failed, job.ambiguous, job.retryable),
                (1, 1, 0, usize::from(!permanent))
            );
            assert!(matches!(job.status, JobStatus::Failed));
            assert_eq!(
                provider.calls.lock().unwrap().len(),
                if permanent { 1 } else { 3 }
            );
            assert!(item(&f, 0).await.tmdb_id.is_none());
        }
    }
    #[tokio::test]
    async fn jobs_share_one_provider_slot_and_cancelled_queue_never_makes_requests() {
        let f = fixture(1);
        let gate = Arc::new(Semaphore::new(0));
        let first = Arc::new(Fake {
            detail_gate: Some(gate.clone()),
            ..Default::default()
        });
        let (_, first_task) = launch(&f, first.clone(), Arc::new(AtomicBool::new(false)));
        first.detail_entered.notified().await;
        let second = Arc::new(Fake::default());
        let (id, second_task) = launch(&f, second.clone(), Arc::new(AtomicBool::new(false)));
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(second.calls.lock().unwrap().is_empty());
        f.state.tmdb_jobs.cancel(&id).unwrap();
        gate.add_permits(1);
        finish(first_task).await;
        finish(second_task).await;
        assert!(second.calls.lock().unwrap().is_empty());
        assert!(matches!(
            f.state.tmdb_jobs.status(&id).unwrap().status,
            JobStatus::Cancelled
        ));
    }
    #[tokio::test]
    async fn credentials_are_private_typed_redacted_rejected_and_deleted() {
        use super::super::{
            CREDENTIAL_FILE, ConfigureTmdb, CredentialType, StoredCredential, clear, configure,
            configured_token, resolve_credential, status,
        };
        use axum::{Json, extract::State};
        let f = fixture(1);
        // A hex-shaped bearer must not silently become a v3 key.
        let secret = "abcdef0123456789abcdef0123456789";
        let (pending, _pending_task) = f
            .state
            .tmdb_jobs
            .create(f.profile.clone(), f.ids.clone())
            .unwrap();
        let returned = configure(
            State(f.state.clone()),
            Json(ConfigureTmdb {
                credential_type: CredentialType::Bearer,
                credential: format!("Bearer {secret}"),
                auto_enrich: true,
                profile_id: Some(f.profile.clone()),
            }),
        )
        .await
        .unwrap();
        let rendered = serde_json::to_string(&returned.0).unwrap();
        assert!(!rendered.contains(secret));
        assert!(rendered.contains("bearer"));
        assert!(rendered.contains("true"));
        assert_eq!(
            configured_token(&f.state).unwrap().0,
            format!("Bearer {secret}")
        );
        assert!(matches!(
            f.state.tmdb_jobs.status(&pending.job_id).unwrap().status,
            JobStatus::Cancelled
        ));
        let path = f.state.config.data_dir.join(CREDENTIAL_FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let status =
            serde_json::to_string(&status(State(f.state.clone())).await.unwrap().0).unwrap();
        assert!(!status.contains(secret));
        assert!(!status.contains("credential\""));
        let prior = std::fs::read(&path).unwrap();
        let invalid = configure(
            State(f.state.clone()),
            Json(ConfigureTmdb {
                credential_type: CredentialType::ApiKey,
                credential: "invalid".into(),
                auto_enrich: false,
                profile_id: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(invalid.1, "TMDB_CREDENTIAL_INVALID");
        assert_eq!(std::fs::read(&path).unwrap(), prior);
        let unknown = configure(
            State(f.state.clone()),
            Json(ConfigureTmdb {
                credential_type: CredentialType::ApiKey,
                credential: secret.into(),
                auto_enrich: true,
                profile_id: Some(uuid::Uuid::new_v4().to_string()),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(unknown.0, axum::http::StatusCode::NOT_FOUND);
        assert_eq!(std::fs::read(&path).unwrap(), prior);
        let fallback = resolve_credential(
            None,
            Some("synthetic-environment-token-without-space".into()),
        )
        .unwrap();
        assert_eq!(fallback.1, "environment");
        let stored = StoredCredential {
            credential_type: CredentialType::ApiKey,
            credential: secret.into(),
            auto_enrich: false,
            auto_profile_id: None,
        };
        assert_eq!(
            resolve_credential(Some(stored), Some("different-environment-secret".into()))
                .unwrap()
                .0,
            secret
        );
        let cleared = clear(State(f.state.clone())).await.unwrap();
        assert!(!path.exists());
        assert!(!serde_json::to_string(&cleared.0).unwrap().contains(secret));
    }
    #[tokio::test]
    async fn interrupted_real_worker_recovers_redacted_state_and_can_retry() {
        let mut f = fixture(1);
        let (id, task) = launch(
            &f,
            Arc::new(Fake::default()),
            Arc::new(AtomicBool::new(true)),
        );
        wait_paused(&f.state.tmdb_jobs, &id).await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let recovered = Arc::new(TmdbJobService::initialize(&f.state.config.data_dir).unwrap());
        let old = recovered.status(&id).unwrap();
        assert!(matches!(old.status, JobStatus::Failed));
        assert_eq!((old.processed, old.retryable), (0, 1));
        f.state.tmdb_jobs = recovered;
        let (retry, task) = launch(
            &f,
            Arc::new(Fake::default()),
            Arc::new(AtomicBool::new(false)),
        );
        finish(task).await;
        assert!(matches!(
            f.state.tmdb_jobs.status(&retry).unwrap().status,
            JobStatus::Completed
        ));
        assert_eq!(item(&f, 0).await.tmdb_id, Some(348));
        let state =
            std::fs::read_to_string(f.state.config.data_dir.join("tmdb-jobs.json")).unwrap();
        assert!(!state.contains("Synthetic worker test"));
        assert!(!state.contains("Provider synopsis"));
        assert!(!state.contains("http"));
        assert!(!state.contains("credential"));
    }
    #[tokio::test]
    async fn portable_export_excludes_stored_tmdb_secret_and_worker_state() {
        use super::super::{ConfigureTmdb, CredentialType, configure};
        use axum::{Json, body::Body, extract::State, http::Request};
        use http_body_util::BodyExt;
        use std::io::{Cursor, Read};
        use tower::ServiceExt;
        let f = fixture(1);
        let secret = "123456789abcdef0123456789abcdef0";
        let _ = configure(
            State(f.state.clone()),
            Json(ConfigureTmdb {
                credential_type: CredentialType::ApiKey,
                credential: secret.into(),
                auto_enrich: false,
                profile_id: None,
            }),
        )
        .await
        .unwrap();
        let app = crate::api::router(f.state.clone());
        let request = |method: &str, path: &str| {
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8443")
                .body(Body::empty())
                .unwrap()
        };
        let response = app
            .clone()
            .oneshot(request("POST", "/api/2026-10-14/admin/exports"))
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
        let started: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let id = started["export_id"].as_str().unwrap();
        let ready = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let status = app
                    .clone()
                    .oneshot(request(
                        "GET",
                        &format!("/api/2026-10-14/admin/exports/{id}/status"),
                    ))
                    .await
                    .unwrap();
                let status: serde_json::Value =
                    serde_json::from_slice(&status.into_body().collect().await.unwrap().to_bytes())
                        .unwrap();
                if status["state"] != "building" {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(ready["state"], "ready");
        let response = app
            .oneshot(request(
                "GET",
                &format!("/api/2026-10-14/admin/exports/{id}"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            assert!(!entry.name().unwrap().contains("tmdb-token"));
            assert!(!entry.name().unwrap().contains("tmdb-jobs"));
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes())
            );
        }
    }
    #[tokio::test]
    async fn explicit_implicit_and_auto_3001_worklists_are_rejected_without_spawn_or_truncation() {
        use super::super::{
            ConfigureTmdb, CredentialType, MAX_STATE_BYTES, MAX_TASKS, StartJob, configure,
            enqueue_auto_checked, start_job,
        };
        use axum::{Json, extract::State, http::StatusCode};
        let f = fixture(3001);
        let _ = configure(
            State(f.state.clone()),
            Json(ConfigureTmdb {
                credential_type: CredentialType::ApiKey,
                credential: "0123456789abcdef0123456789abcdef".into(),
                auto_enrich: true,
                profile_id: Some(f.profile.clone()),
            }),
        )
        .await
        .unwrap();
        for explicit in [false, true] {
            let error = start_job(
                State(f.state.clone()),
                Ok(Json(StartJob {
                    profile_id: f.profile.clone(),
                    media_ids: explicit.then(|| f.ids.clone()),
                })),
            )
            .await
            .err()
            .unwrap();
            assert_eq!(
                (error.0, error.1),
                (StatusCode::PAYLOAD_TOO_LARGE, "TMDB_JOB_TOO_LARGE")
            );
        }
        let error = enqueue_auto_checked(f.state.clone()).await.err().unwrap();
        assert_eq!(
            (error.0, error.1),
            (StatusCode::PAYLOAD_TOO_LARGE, "TMDB_JOB_TOO_LARGE")
        );
        assert!(f.state.tmdb_jobs.jobs.lock().unwrap().is_empty());
        assert_eq!(f.state.tmdb_jobs.task_slots.available_permits(), MAX_TASKS);
        assert!(
            std::fs::metadata(f.state.config.data_dir.join("tmdb-jobs.json"))
                .unwrap()
                .len()
                <= MAX_STATE_BYTES as u64
        );
    }
    #[tokio::test]
    async fn repeated_cancellation_start_retry_and_auto_cannot_replace_owned_task_slots() {
        use super::super::{
            ConfigureTmdb, CredentialType, MAX_STATE_BYTES, MAX_TASKS, StartJob, configure,
            enqueue_auto_checked, retry_job, start_job,
        };
        use axum::{
            Json,
            extract::{Path, State},
            http::StatusCode,
        };
        let f = fixture(1);
        let _ = configure(
            State(f.state.clone()),
            Json(ConfigureTmdb {
                credential_type: CredentialType::ApiKey,
                credential: "0123456789abcdef0123456789abcdef".into(),
                auto_enrich: true,
                profile_id: Some(f.profile.clone()),
            }),
        )
        .await
        .unwrap();
        let gate = Arc::new(Semaphore::new(0));
        let first = Arc::new(Fake {
            detail_gate: Some(gate.clone()),
            ..Default::default()
        });
        let (a, a_task) = launch(&f, first.clone(), Arc::new(AtomicBool::new(false)));
        first.detail_entered.notified().await;
        let second = Arc::new(Fake::default());
        let (b, b_task) = launch(&f, second.clone(), Arc::new(AtomicBool::new(false)));
        for _ in 0..20 {
            f.state.tmdb_jobs.cancel(&a).unwrap();
            f.state.tmdb_jobs.cancel(&b).unwrap();
            let errors = [
                start_job(
                    State(f.state.clone()),
                    Ok(Json(StartJob {
                        profile_id: f.profile.clone(),
                        media_ids: Some(f.ids.clone()),
                    })),
                )
                .await
                .err()
                .unwrap(),
                retry_job(State(f.state.clone()), Path(a.clone()))
                    .await
                    .err()
                    .unwrap(),
                enqueue_auto_checked(f.state.clone()).await.err().unwrap(),
            ];
            for error in errors {
                assert_eq!(
                    (error.0, error.1),
                    (StatusCode::TOO_MANY_REQUESTS, "TMDB_JOB_BUSY")
                );
            }
            assert!(!a_task.is_finished());
            assert!(!b_task.is_finished());
            assert_eq!(f.state.tmdb_jobs.task_slots.available_permits(), 0);
            assert_eq!(f.state.tmdb_jobs.inflight.lock().unwrap().len(), MAX_TASKS);
            assert_eq!(f.state.tmdb_jobs.jobs.lock().unwrap().len(), MAX_TASKS);
            assert!(
                std::fs::metadata(f.state.config.data_dir.join("tmdb-jobs.json"))
                    .unwrap()
                    .len()
                    <= MAX_STATE_BYTES as u64
            );
        }
        gate.add_permits(1);
        finish(a_task).await;
        finish(b_task).await;
        assert_eq!(f.state.tmdb_jobs.task_slots.available_permits(), MAX_TASKS);
        assert!(second.calls.lock().unwrap().is_empty());
        assert!(item(&f, 0).await.tmdb_id.is_none());
    }
    #[tokio::test]
    async fn retry_history_stays_at_eight_and_expired_records_are_pruned_durably_on_restart_or_read()
     {
        use super::super::{
            HISTORY_TTL, MAX_HISTORY, MAX_STATE_BYTES, MAX_TASKS, now_seconds, retry_job,
        };
        use axum::extract::{Path, State};
        let f = fixture(0);
        let (job, guard) = f
            .state
            .tmdb_jobs
            .create(f.profile.clone(), Vec::new())
            .unwrap();
        f.state.tmdb_jobs.cancel(&job.job_id).unwrap();
        drop(guard);
        let mut prior = job.job_id;
        for _ in 0..24 {
            let (_, view) = retry_job(State(f.state.clone()), Path(prior))
                .await
                .unwrap();
            prior = view.0.job_id;
            f.state.tmdb_jobs.cancel(&prior).unwrap();
            tokio::time::timeout(Duration::from_secs(2), async {
                while f.state.tmdb_jobs.task_slots.available_permits() != MAX_TASKS {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert!(f.state.tmdb_jobs.jobs.lock().unwrap().len() <= MAX_HISTORY);
            assert!(
                std::fs::metadata(f.state.config.data_dir.join("tmdb-jobs.json"))
                    .unwrap()
                    .len()
                    <= MAX_STATE_BYTES as u64
            );
        }
        let recovered = TmdbJobService::initialize(&f.state.config.data_dir).unwrap();
        assert_eq!(recovered.jobs.lock().unwrap().len(), MAX_HISTORY);
        assert_eq!(recovered.task_slots.available_permits(), MAX_TASKS);
        {
            let mut jobs = recovered.jobs.lock().unwrap();
            for job in jobs.values_mut() {
                job.finished_at = Some(now_seconds() - HISTORY_TTL.as_secs());
            }
        }
        assert!(recovered.status(&prior).is_none());
        assert!(recovered.jobs.lock().unwrap().is_empty());
        assert_eq!(
            std::fs::read_to_string(f.state.config.data_dir.join("tmdb-jobs.json")).unwrap(),
            "{}"
        );
        let recovered = TmdbJobService::initialize(&f.state.config.data_dir).unwrap();
        assert!(recovered.jobs.lock().unwrap().is_empty());
    }
    #[test]
    fn startup_streams_and_prunes_old_history_and_preserves_oversized_or_malformed_state() {
        use super::super::{MAX_HISTORY, MAX_STATE_BYTES, MAX_WORKLIST, now_seconds};
        let f = fixture(0);
        let (seed, guard) = f
            .state
            .tmdb_jobs
            .create(f.profile.clone(), Vec::new())
            .unwrap();
        drop(guard);
        let path = f.state.config.data_dir.join("tmdb-jobs.json");
        let now = now_seconds();
        let mut history = std::collections::BTreeMap::new();
        for n in 0..30 {
            let mut job = seed.clone();
            job.job_id = format!("legacy-{n:02}");
            job.status = JobStatus::Completed;
            job.retryable = 0;
            job.created_at = now - n;
            job.finished_at = Some(now - n);
            history.insert(job.job_id.clone(), job);
        }
        std::fs::write(&path, serde_json::to_vec(&history).unwrap()).unwrap();
        let recovered = TmdbJobService::initialize(&f.state.config.data_dir).unwrap();
        assert_eq!(recovered.jobs.lock().unwrap().len(), MAX_HISTORY);
        assert!(recovered.status("legacy-00").is_some());
        assert!(recovered.status("legacy-29").is_none());
        assert!(std::fs::metadata(&path).unwrap().len() <= MAX_STATE_BYTES as u64);
        let mut malformed = seed.clone();
        malformed.media_ids = vec!["m".into(); MAX_WORKLIST + 1];
        malformed.total = malformed.media_ids.len();
        malformed.retryable = 0;
        let bytes = serde_json::to_vec(&std::collections::BTreeMap::from([(
            malformed.job_id.clone(),
            malformed,
        )]))
        .unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let error = TmdbJobService::initialize(&f.state.config.data_dir)
            .err()
            .unwrap();
        assert!(error.contains("limites"));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let file = std::fs::File::create(&path).unwrap();
        file.set_len((MAX_STATE_BYTES + 1) as u64).unwrap();
        drop(file);
        assert!(TmdbJobService::initialize(&f.state.config.data_dir).is_err());
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            (MAX_STATE_BYTES + 1) as u64
        );
    }
    #[test]
    fn failed_persistence_refuses_creation_before_mutation_and_returns_its_reservation() {
        use super::super::MAX_TASKS;
        let f = fixture(1);
        let original = std::fs::read(f.state.config.data_dir.join("tmdb-jobs.json")).unwrap();
        let obstruction = f.state.config.data_dir.join("not-a-directory");
        std::fs::write(&obstruction, b"preserved").unwrap();
        *f.state.tmdb_jobs.state_path.lock().unwrap() = Some(obstruction.join("tmdb-jobs.json"));
        let error = f
            .state
            .tmdb_jobs
            .create(f.profile.clone(), f.ids.clone())
            .err()
            .unwrap();
        assert_eq!(error.1, "TMDB_JOB_UNAVAILABLE");
        assert!(f.state.tmdb_jobs.jobs.lock().unwrap().is_empty());
        assert!(f.state.tmdb_jobs.inflight.lock().unwrap().is_empty());
        assert_eq!(f.state.tmdb_jobs.task_slots.available_permits(), MAX_TASKS);
        assert_eq!(
            std::fs::read(f.state.config.data_dir.join("tmdb-jobs.json")).unwrap(),
            original
        );
        assert_eq!(std::fs::read(obstruction).unwrap(), b"preserved");
    }
    #[tokio::test]
    async fn oversized_http_worklist_is_a_redacted_413_before_large_vector_or_worker_creation() {
        use super::super::MAX_WORKLIST;
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use http_body_util::BodyExt;
        use tower::ServiceExt;
        let f = fixture(0);
        let body = serde_json::json!({"profile_id":f.profile,"media_ids":vec!["m";MAX_WORKLIST+1]})
            .to_string();
        let response = crate::api::router(f.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/2026-10-14/admin/tmdb/jobs")
                    .header("host", "localhost:8443")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(body["code"], "TMDB_JOB_TOO_LARGE");
        assert!(f.state.tmdb_jobs.jobs.lock().unwrap().is_empty());
    }
    #[test]
    fn serialized_state_budget_refuses_before_map_or_disk_mutation() {
        use super::super::{MAX_HISTORY, MAX_STATE_BYTES, MAX_TASKS, MAX_WORKLIST};
        let f = fixture(0);
        let ids = (0..MAX_WORKLIST)
            .map(|n| format!("{n:04}{}", "x".repeat(124)))
            .collect::<Vec<_>>();
        let path = f.state.config.data_dir.join("tmdb-jobs.json");
        let mut rejected = false;
        for _ in 0..=MAX_HISTORY {
            let prior = std::fs::read(&path).unwrap();
            let count = f.state.tmdb_jobs.jobs.lock().unwrap().len();
            match f.state.tmdb_jobs.create(f.profile.clone(), ids.clone()) {
                Ok((_, guard)) => drop(guard),
                Err(error) => {
                    assert_eq!(error.1, "TMDB_JOB_BUSY");
                    assert_eq!(std::fs::read(&path).unwrap(), prior);
                    assert_eq!(f.state.tmdb_jobs.jobs.lock().unwrap().len(), count);
                    rejected = true;
                    break;
                }
            }
            assert!(std::fs::metadata(&path).unwrap().len() <= MAX_STATE_BYTES as u64);
        }
        assert!(rejected);
        assert_eq!(f.state.tmdb_jobs.task_slots.available_permits(), MAX_TASKS);
        assert!(f.state.tmdb_jobs.inflight.lock().unwrap().is_empty());
        assert!(std::fs::metadata(&path).unwrap().len() <= MAX_STATE_BYTES as u64);
    }
}
