//! Volatile playback coordination; mutations are serialized by the SQLite actor.
use crate::{
    api::{ApiError, AppState},
    media_stream::{FileVersion, io_job, open_indexed},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chill_domain::{
    AcquirePlayback, Lease, PersonalState, PlaybackControl, PlaybackGrant, Progress,
    ReleasePlayback, valid_speed,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

const LEASE_TIME: Duration = Duration::from_secs(30);
const MAX_ACTIVE_PROFILES: usize = 3;
type Failure = &'static str;

#[derive(Clone)]
pub struct PlaybackService {
    book: Arc<Mutex<LeaseBook>>,
    pub(crate) io: Arc<Semaphore>,
    pub(crate) streams: Arc<Semaphore>,
}
impl Default for PlaybackService {
    fn default() -> Self {
        Self {
            book: Arc::new(Mutex::new(LeaseBook::new())),
            io: Arc::new(Semaphore::new(4)),
            streams: Arc::new(Semaphore::new(8)),
        }
    }
}
#[derive(Clone)]
pub(crate) struct StreamLease {
    pub session: String,
    pub generation: u64,
    pub media: String,
    pub root: PathBuf,
    pub path: PathBuf,
    pub root_revision: u64,
    pub version: FileVersion,
}
struct Entry {
    lease: Lease,
    stream: StreamLease,
    deadline: Instant,
    sequence: u64,
    sample: Option<(Progress, Instant)>,
}
struct LeaseBook {
    epoch: String,
    generation: u64,
    entries: BTreeMap<String, Entry>,
}
impl LeaseBook {
    fn new() -> Self {
        Self {
            epoch: uuid::Uuid::new_v4().to_string(),
            generation: 0,
            entries: BTreeMap::new(),
        }
    }
    fn acquire(
        &mut self,
        profile: String,
        request: AcquirePlayback,
        mut stream: StreamLease,
        now: Instant,
    ) -> Result<Lease, Failure> {
        self.entries.retain(|_, e| now < e.deadline);
        if self.entries.contains_key(&profile) {
            return Err("PLAYBACK_ACTIVE");
        }
        if self.entries.len() >= MAX_ACTIVE_PROFILES {
            return Err("PLAYBACK_CAPACITY");
        }
        self.generation = self
            .generation
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or("PLAYBACK_CAPACITY")?;
        let session = uuid::Uuid::new_v4().to_string();
        stream.session = session.clone();
        stream.generation = self.generation;
        stream.media = request.media_id.clone();
        let lease = Lease {
            server_epoch: self.epoch.clone(),
            session_id: session,
            profile_id: profile.clone(),
            device_id: request.device_id,
            media_id: request.media_id,
            media_revision: stream.version.revision(stream.root_revision),
            generation: self.generation,
            remaining_ms: 30_000,
            transferring: false,
            context: request.context,
        };
        self.entries.insert(
            profile,
            Entry {
                lease: lease.clone(),
                stream,
                deadline: now + LEASE_TIME,
                sequence: 0,
                sample: None,
            },
        );
        Ok(lease)
    }
    fn entry(&self, profile: &str, c: &PlaybackControl, now: Instant) -> Result<&Entry, Failure> {
        let e = self.entries.get(profile).ok_or("LEASE_INVALID")?;
        if now >= e.deadline
            || e.lease.session_id != c.session_id
            || e.lease.generation != c.generation
            || e.lease.media_id != c.media_id
            || e.lease.media_revision != c.media_revision
        {
            return Err("LEASE_INVALID");
        }
        Ok(e)
    }
    fn heartbeat(&mut self, p: &str, c: &PlaybackControl, now: Instant) -> Result<Lease, Failure> {
        self.entry(p, c, now)?;
        let e = self.entries.get_mut(p).expect("validated entry");
        e.deadline = now + LEASE_TIME;
        let mut l = e.lease.clone();
        l.remaining_ms = 30_000;
        Ok(l)
    }
    fn validate_progress(
        &self,
        p: &str,
        event: &Progress,
        now: Instant,
    ) -> Result<Duration, Failure> {
        let e = self.entry(p, &control(event), now)?;
        if event.sequence <= e.sequence
            || event.sequence > i64::MAX as u64
            || event.duration_ms == 0
            || event.duration_ms > i64::MAX as u64
            || event.position_ms > event.duration_ms
            || !valid_speed(event.speed)
        {
            return Err("PROGRESS_INVALID");
        }
        let mut maximum = 0;
        if let Some((last, at)) = &e.sample {
            let elapsed = now
                .saturating_duration_since(*at)
                .as_millis()
                .min(u64::MAX as u128) as u64;
            let boundary = chill_domain::final_tenth_start(event.duration_ms);
            if last.playing
                && !last.buffering
                && !last.seek
                && event.playing
                && !event.buffering
                && !event.seek
                && last.duration_ms == event.duration_ms
                && last.speed == event.speed
                && event.position_ms >= last.position_ms
                && event.position_ms >= boundary
            {
                let advance = event.position_ms - last.position_ms;
                if advance as f64 <= elapsed as f64 * event.speed + 1000.0 {
                    let final_distance = event
                        .position_ms
                        .saturating_sub(last.position_ms.max(boundary));
                    maximum = elapsed.min((final_distance as f64 / event.speed).floor() as u64);
                }
            }
        }
        if event.final_elapsed_ms > maximum {
            return Err("PROGRESS_INVALID");
        }
        Ok(e.deadline.saturating_duration_since(now))
    }
    fn record(&mut self, p: &str, event: Progress, now: Instant) {
        if let Some(e) = self.entries.get_mut(p) {
            e.sequence = event.sequence;
            e.sample = Some((event, now));
        }
    }
}
fn control(event: &Progress) -> PlaybackControl {
    PlaybackControl {
        session_id: event.session_id.clone(),
        generation: event.generation,
        media_id: event.media_id.clone(),
        media_revision: event.media_revision,
    }
}
fn failure(code: Failure) -> ApiError {
    ApiError(
        match code {
            "PLAYBACK_ACTIVE" | "LEASE_INVALID" => StatusCode::CONFLICT,
            "PLAYBACK_CAPACITY" | "MEDIA_BUSY" => StatusCode::SERVICE_UNAVAILABLE,
            "MEDIA_CHANGED" => StatusCode::PRECONDITION_FAILED,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        },
        code,
    )
}
impl PlaybackService {
    pub(crate) fn clear(&self) {
        if let Ok(mut b) = self.book.lock() {
            b.entries.clear();
        }
    }
    pub(crate) fn invalidate_stream(&self, s: &StreamLease) {
        if let Ok(mut b) = self.book.lock() {
            b.entries.retain(|_, e| {
                e.lease.session_id != s.session || e.lease.generation != s.generation
            });
        }
    }
    pub(crate) fn revoke_profile(&self, p: &str) {
        if let Ok(mut b) = self.book.lock() {
            b.entries.remove(p);
        }
    }
    pub(crate) fn reset_final(&self, p: &str, m: &str) {
        if let Ok(mut b) = self.book.lock()
            && let Some(e) = b.entries.get_mut(p)
            && e.lease.media_id == m
        {
            e.sample = None;
        }
    }
    pub(crate) fn stream_lease(
        &self,
        session: &str,
        generation: u64,
        media: &str,
    ) -> Result<StreamLease, ApiError> {
        let b = self.book.lock().map_err(|_| failure("MEDIA_BUSY"))?;
        b.entries
            .values()
            .find(|e| {
                e.lease.session_id == session
                    && e.lease.generation == generation
                    && e.lease.media_id == media
                    && Instant::now() < e.deadline
            })
            .map(|e| e.stream.clone())
            .ok_or(failure("LEASE_INVALID"))
    }
    pub(crate) fn check_stream(&self, s: &StreamLease) -> Result<(), ApiError> {
        self.stream_lease(&s.session, s.generation, &s.media)
            .map(|_| ())
    }
    fn progress_budget(&self, p: &str, e: &Progress) -> Result<Duration, ApiError> {
        self.book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .validate_progress(p, e, Instant::now())
            .map_err(failure)
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/profiles/{profile}/playback/acquire", post(acquire))
        .route(
            "/api/v1/profiles/{profile}/playback/heartbeat",
            post(heartbeat),
        )
        .route(
            "/api/v1/profiles/{profile}/playback/progress",
            post(progress),
        )
        .route("/api/v1/profiles/{profile}/playback/release", post(release))
        .route(
            "/api/v1/profiles/{profile}/playback/speed",
            post(save_speed),
        )
        .route(
            "/api/v1/profiles/{profile}/media/{media}/state",
            get(personal),
        )
        .route(
            "/api/v1/media/{media}/stream",
            get(crate::media_stream::stream),
        )
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpeedChange {
    lease: PlaybackControl,
    speed: f64,
}
async fn save_speed(
    State(s): State<AppState>,
    Path(p): Path<String>,
    Json(change): Json<SpeedChange>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = s.playback.clone();
    if !valid_speed(change.speed) {
        return Err(failure("PLAYBACK_REQUEST_INVALID"));
    }
    s.database
        .call(move |store| {
            service
                .book
                .lock()
                .map_err(|_| "INVALID_OPERATION")?
                .entry(&p, &change.lease, Instant::now())
                .map_err(str::to_owned)?;
            let media = store.media(&p, &change.lease.media_id)?;
            if let Some(series) = media.series {
                store.set_series_speed(&p, &series, change.speed)?;
            }
            Ok(())
        })
        .await?;
    Ok(Json(serde_json::json!({"saved":true})))
}

async fn acquire(
    State(s): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<AcquirePlayback>,
) -> Result<Json<PlaybackGrant>, ApiError> {
    if uuid::Uuid::parse_str(&profile).is_err()
        || uuid::Uuid::parse_str(&body.media_id).is_err()
        || uuid::Uuid::parse_str(&body.device_id).is_err()
        || body.context.kind != "standalone"
        || body.context.id.is_some()
    {
        return Err(failure("PLAYBACK_REQUEST_INVALID"));
    }
    let p = profile.clone();
    let m = body.media_id.clone();
    let path = s
        .database
        .call(move |store| {
            if !store.media(&p, &m)?.available {
                return Ok(None);
            }
            store.media_path(&m).map(Some)
        })
        .await?
        .ok_or(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "MEDIA_UNAVAILABLE",
        ))?;
    let settings = s.library.lock().map_err(|_| failure("MEDIA_BUSY"))?.clone();
    let root = settings
        .video_root
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "ROOT_REQUIRED"))?;
    let r = root.clone();
    let f = path.clone();
    let (_, version) = io_job(&s.playback, move || open_indexed(&r, &f)).await?;
    if version.size == 0 {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "MEDIA_UNAVAILABLE",
        ));
    }
    let service = s.playback.clone();
    let library = s.library.clone();
    let result = s
        .database
        .call(move |store| {
            // Revalidate after potentially slow file IO: deletion/root changes must
            // not grant a lease based on stale preflight data.
            if store.media_path(&body.media_id)? != path
                || !store.media(&profile, &body.media_id)?.available
            {
                return Ok(Err("MEDIA_CHANGED"));
            }
            // Retain the library lock through publication. A root update holds
            // this same lock while revoking leases (library -> book order), so
            // it cannot clear the book and then let an old-root grant appear.
            let current_settings = library.lock().map_err(|_| "INVALID_OPERATION")?;
            if current_settings.revision != settings.revision {
                return Ok(Err("MEDIA_CHANGED"));
            }
            let personal = store.personal_state(&profile, &body.media_id)?;
            let mut book = service.book.lock().map_err(|_| "INVALID_OPERATION")?;
            let lease = book.acquire(
                profile,
                body.clone(),
                StreamLease {
                    session: String::new(),
                    generation: 0,
                    media: body.media_id.clone(),
                    root,
                    path,
                    root_revision: settings.revision,
                    version,
                },
                Instant::now(),
            );
            Ok(lease.map(|lease| PlaybackGrant { lease, personal }))
        })
        .await?;
    Ok(Json(result.map_err(failure)?))
}
async fn heartbeat(
    State(s): State<AppState>,
    Path(p): Path<String>,
    Json(c): Json<PlaybackControl>,
) -> Result<Json<Lease>, ApiError> {
    let service = s.playback.clone();
    let result = s
        .database
        .call(move |store| {
            store.personal_state(&p, &c.media_id)?;
            Ok(service
                .book
                .lock()
                .map_err(|_| "INVALID_OPERATION")?
                .heartbeat(&p, &c, Instant::now()))
        })
        .await?;
    Ok(Json(result.map_err(failure)?))
}
async fn progress(
    State(s): State<AppState>,
    Path(p): Path<String>,
    Json(e): Json<Progress>,
) -> Result<Json<PersonalState>, ApiError> {
    let budget = s
        .playback
        .progress_budget(&p, &e)?
        .min(Duration::from_secs(10));
    let service = s.playback.clone();
    let result = s
        .database
        .call_with_budget(budget, move |store| {
            let now = Instant::now();
            if let Err(code) = service
                .book
                .lock()
                .map_err(|_| "INVALID_OPERATION")?
                .validate_progress(&p, &e, now)
            {
                return Ok(Err(code));
            }
            let saved = store.save_validated_progress(&p, &e.media_id, &e)?;
            service
                .book
                .lock()
                .map_err(|_| "INVALID_OPERATION")?
                .record(&p, e, now);
            Ok(Ok(saved))
        })
        .await?;
    Ok(Json(result.map_err(failure)?))
}
async fn release(
    State(s): State<AppState>,
    Path(p): Path<String>,
    Json(r): Json<ReleasePlayback>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(e) = &r.progress
        && (e.session_id != r.lease.session_id
            || e.generation != r.lease.generation
            || e.media_id != r.lease.media_id
            || e.media_revision != r.lease.media_revision)
    {
        return Err(failure("LEASE_INVALID"));
    }
    let service = s.playback.clone();
    let remaining = {
        let b = service.book.lock().map_err(|_| failure("MEDIA_BUSY"))?;
        b.entry(&p, &r.lease, Instant::now())
            .map_err(failure)?
            .deadline
            .saturating_duration_since(Instant::now())
    };
    let result = s
        .database
        .call_with_budget(remaining.min(Duration::from_secs(10)), move |store| {
            let now = Instant::now();
            if let Err(code) = service
                .book
                .lock()
                .map_err(|_| "INVALID_OPERATION")?
                .entry(&p, &r.lease, now)
            {
                return Ok(Err(code));
            }
            if let Some(e) = r.progress {
                if let Err(code) = service
                    .book
                    .lock()
                    .map_err(|_| "INVALID_OPERATION")?
                    .validate_progress(&p, &e, now)
                {
                    return Ok(Err(code));
                }
                store.save_validated_progress(&p, &e.media_id, &e)?;
            }
            service.revoke_profile(&p);
            Ok(Ok(()))
        })
        .await?;
    result.map_err(failure)?;
    Ok(Json(serde_json::json!({"released":true})))
}
async fn personal(
    State(s): State<AppState>,
    Path((p, m)): Path<(String, String)>,
) -> Result<Json<PersonalState>, ApiError> {
    Ok(Json(
        s.database
            .call(move |store| store.personal_state(&p, &m))
            .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prepared() -> (LeaseBook, Instant, AcquirePlayback, StreamLease) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let path = root.join("x.mp4");
        std::fs::write(&path, b"video").unwrap();
        let (_, version) = open_indexed(&root, &path).unwrap();
        (
            LeaseBook::new(),
            Instant::now(),
            AcquirePlayback {
                media_id: uuid::Uuid::new_v4().to_string(),
                device_id: uuid::Uuid::new_v4().to_string(),
                context: Default::default(),
            },
            StreamLease {
                session: String::new(),
                generation: 0,
                media: String::new(),
                root,
                path,
                root_revision: 1,
                version,
            },
        )
    }
    fn ctrl(l: &Lease) -> PlaybackControl {
        PlaybackControl {
            session_id: l.session_id.clone(),
            generation: l.generation,
            media_id: l.media_id.clone(),
            media_revision: l.media_revision,
        }
    }
    fn event(l: &Lease, sequence: u64, position: u64, credit: u64) -> Progress {
        Progress {
            session_id: l.session_id.clone(),
            generation: l.generation,
            media_id: l.media_id.clone(),
            media_revision: l.media_revision,
            sequence,
            position_ms: position,
            duration_ms: 100_000,
            speed: 1.0,
            final_elapsed_ms: credit,
            playing: true,
            buffering: false,
            seek: false,
        }
    }
    #[test]
    fn exclusivity_expiry_and_restart() {
        let (mut b, t, r, s) = prepared();
        let l = b.acquire("p".into(), r.clone(), s.clone(), t).unwrap();
        assert_eq!(
            b.acquire("p".into(), r.clone(), s.clone(), t).unwrap_err(),
            "PLAYBACK_ACTIVE"
        );
        assert!(b.entry("other", &ctrl(&l), t).is_err());
        assert!(b.entry("p", &ctrl(&l), t + LEASE_TIME).is_err());
        let newer = b.acquire("p".into(), r, s, t + LEASE_TIME).unwrap();
        assert!(newer.generation > l.generation);
        assert!(b.entry("p", &ctrl(&l), t + LEASE_TIME).is_err());
        assert!(LeaseBook::new().entry("p", &ctrl(&newer), t).is_err());
    }
    #[test]
    fn credit_requires_real_elapsed_and_continuous_final_window() {
        let (mut b, t, r, s) = prepared();
        let l = b.acquire("p".into(), r, s, t).unwrap();
        assert!(
            b.validate_progress("p", &event(&l, 1, 90_000, 1), t)
                .is_err()
        );
        b.record("p", event(&l, 1, 90_000, 0), t);
        assert!(
            b.validate_progress(
                "p",
                &event(&l, 2, 100_000, 10_000),
                t + Duration::from_secs(10)
            )
            .is_ok()
        );
        assert!(
            b.validate_progress("p", &event(&l, 2, 100_000, 10_000), t)
                .is_err()
        );
        let mut pause = event(&l, 2, 100_000, 1);
        pause.playing = false;
        assert!(
            b.validate_progress("p", &pause, t + Duration::from_secs(10))
                .is_err()
        );
        let mut seek = pause.clone();
        seek.playing = true;
        seek.seek = true;
        assert!(
            b.validate_progress("p", &seek, t + Duration::from_secs(10))
                .is_err()
        );
        assert!(
            b.validate_progress("p", &event(&l, 1, 90_000, 0), t)
                .is_err()
        );
    }
    #[test]
    fn heartbeat_binds_revision_and_old_generation() {
        let (mut b, t, r, s) = prepared();
        let l = b.acquire("p".into(), r, s, t).unwrap();
        let mut c = ctrl(&l);
        c.media_revision += 1;
        assert!(b.heartbeat("p", &c, t).is_err());
        assert!(
            b.heartbeat("p", &ctrl(&l), t + Duration::from_secs(29))
                .is_ok()
        );
        assert!(b.entry("p", &ctrl(&l), t + Duration::from_secs(31)).is_ok());
    }

    #[test]
    fn credits_are_prorated_and_use_real_time_at_every_speed() {
        for (speed, seconds, credit) in [(0.5, 20, 20_000), (1.0, 10, 10_000), (2.0, 5, 5_000)] {
            let (mut b, t, r, s) = prepared();
            let l = b.acquire("p".into(), r, s, t).unwrap();
            let mut first = event(&l, 1, 90_000, 0);
            first.speed = speed;
            b.record("p", first, t);
            let mut last = event(&l, 2, 100_000, credit);
            last.speed = speed;
            let now = t + Duration::from_secs(seconds);
            assert!(b.validate_progress("p", &last, now).is_ok());
            last.final_elapsed_ms += 1;
            assert!(b.validate_progress("p", &last, now).is_err());
        }
        let (mut b, t, r, s) = prepared();
        let l = b.acquire("p".into(), r, s, t).unwrap();
        b.record("p", event(&l, 1, 85_000, 0), t);
        assert!(
            b.validate_progress(
                "p",
                &event(&l, 2, 95_000, 5_000),
                t + Duration::from_secs(10)
            )
            .is_ok()
        );
        assert!(
            b.validate_progress(
                "p",
                &event(&l, 2, 95_000, 5_001),
                t + Duration::from_secs(10)
            )
            .is_err()
        );
    }

    #[test]
    fn profile_capacity_is_bounded_and_expired_entries_are_reusable() {
        let (mut b, t, r, s) = prepared();
        for p in ["a", "b", "c"] {
            b.acquire(p.into(), r.clone(), s.clone(), t).unwrap();
        }
        assert_eq!(
            b.acquire("d".into(), r.clone(), s.clone(), t).unwrap_err(),
            "PLAYBACK_CAPACITY"
        );
        assert!(b.acquire("d".into(), r, s, t + LEASE_TIME).is_ok());
        assert_eq!(b.entries.len(), 1);
    }

    #[test]
    fn buffering_and_unknown_duration_cannot_add_watch_credit() {
        let (mut b, t, r, s) = prepared();
        let l = b.acquire("p".into(), r, s, t).unwrap();
        let mut e = event(&l, 1, 0, 0);
        e.duration_ms = 0;
        assert_eq!(
            b.validate_progress("p", &e, t).unwrap_err(),
            "PROGRESS_INVALID"
        );
        b.record("p", event(&l, 1, 90_000, 0), t);
        let mut buffered = event(&l, 2, 95_000, 1);
        buffered.buffering = true;
        assert!(
            b.validate_progress("p", &buffered, t + Duration::from_secs(5))
                .is_err()
        );
        buffered.final_elapsed_ms = 0;
        assert!(
            b.validate_progress("p", &buffered, t + Duration::from_secs(5))
                .is_ok()
        );
        b.record("p", buffered, t + Duration::from_secs(5));
        assert!(
            b.validate_progress("p", &event(&l, 3, 100_000, 1), t + Duration::from_secs(10))
                .is_err()
        );
        assert!(
            b.validate_progress("p", &event(&l, 3, 100_000, 0), t + Duration::from_secs(10))
                .is_ok()
        );
    }
}
