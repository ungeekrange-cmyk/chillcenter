//! Volatile playback coordination; mutations are serialized by the SQLite actor.
use crate::{
    api::{ApiError, AppState},
    media_stream::{FileVersion, io_job, open_indexed},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
};
use chill_domain::{
    AcquirePlayback, Lease, PersonalState, PlaybackControl, PlaybackGrant, Progress,
    ReleasePlayback, TransferAck, TransferOffer, TransferPhase, TransferStatus, valid_speed,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, broadcast};

const LEASE_TIME: Duration = Duration::from_secs(30);
#[allow(dead_code)]
const TRANSFER_OFFER_TIME: Duration = Duration::from_secs(120);
#[allow(dead_code)]
const MAX_TRANSFER_TOKENS: usize = 6;
const MAX_ACTIVE_PROFILES: usize = 3;
type Failure = &'static str;

#[derive(Clone)]
pub struct PlaybackService {
    book: Arc<Mutex<LeaseBook>>,
    pub(crate) io: Arc<Semaphore>,
    pub(crate) streams: Arc<Semaphore>,
    events: broadcast::Sender<PauseEvent>,
    event_subscribers: Arc<Semaphore>,
}
#[derive(Clone)]
struct PauseEvent {
    profile: String,
    session: String,
    generation: u64,
    token: String,
    remaining_ms: u64,
}
impl Default for PlaybackService {
    fn default() -> Self {
        let (events, _) = broadcast::channel(MAX_TRANSFER_TOKENS);
        Self {
            book: Arc::new(Mutex::new(LeaseBook::new())),
            io: Arc::new(Semaphore::new(4)),
            streams: Arc::new(Semaphore::new(8)),
            events,
            event_subscribers: Arc::new(Semaphore::new(MAX_TRANSFER_TOKENS)),
        }
    }
}
impl PlaybackService {
    /// Fingerprint workers must stop issuing reads while any live playback
    /// lease exists. Expired leases do not keep maintenance suspended.
    pub(crate) fn fingerprint_should_pause(&self) -> bool {
        let now = Instant::now();
        self.book.lock().map_or(true, |book| {
            book.entries.values().any(|entry| now < entry.deadline)
        })
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
    short_tail_contiguous: bool,
    pending: Option<PendingTransfer>,
}

/// Short videos must reach their end after continuously covering the final tenth.
/// This bounded, volatile proof belongs to the validated lease, not the wire DTO
/// or the persisted real-time credit (which stays in milliseconds, at every speed).
fn short_tail_continues(entry: &Entry, event: &Progress, now: Instant) -> bool {
    if event.duration_ms == 0 || event.duration_ms >= 100_000 {
        return false;
    }
    if event.position_ms <= chill_domain::final_tenth_start(event.duration_ms) {
        return true;
    }
    let Some((last, at)) = &entry.sample else {
        return false;
    };
    if !entry.short_tail_contiguous
        || last.duration_ms != event.duration_ms
        || event.seek
        || last.seek
        || event.position_ms < last.position_ms
    {
        return false;
    }
    // A stationary pause/buffer/speed change preserves already covered media.
    if event.position_ms == last.position_ms {
        return true;
    }
    // Web clients report EOF paused; the final advancing interval still began
    // playing. Other paused/buffering advances must not supply coverage.
    let ended = event.position_ms == event.duration_ms;
    let elapsed = now.saturating_duration_since(*at).as_millis() as f64;
    last.playing
        && !last.buffering
        && !event.buffering
        && (event.playing || ended)
        && last.speed == event.speed
        && elapsed > 0.0
        && (event.position_ms - last.position_ms) as f64 <= elapsed * event.speed + 1000.0
}

fn save_progress(
    store: &chill_domain::Store,
    service: &PlaybackService,
    profile: &str,
    event: &Progress,
    now: Instant,
) -> Result<PersonalState, String> {
    let complete = service
        .book
        .lock()
        .map_err(|_| "INVALID_OPERATION")?
        .completed_short_tail(profile, event, now);
    store.save_validated_progress_with_short_tail(profile, &event.media_id, event, complete)
}
#[allow(dead_code)]
#[derive(Clone)]
struct PendingTransfer {
    token: String,
    target: AcquirePlayback,
    stream: StreamLease,
    deadline: Instant,
    acknowledged: bool,
}
#[allow(dead_code)]
#[derive(Clone)]
struct Offer {
    profile: String,
    generation: u64,
    target: AcquirePlayback,
    stream: StreamLease,
    expires: Instant,
}
struct LeaseBook {
    epoch: String,
    generation: u64,
    entries: BTreeMap<String, Entry>,
    offers: BTreeMap<String, Offer>,
    #[allow(dead_code)]
    completed: BTreeMap<String, TransferStatus>,
}
#[allow(dead_code)]
impl LeaseBook {
    fn new() -> Self {
        Self {
            epoch: uuid::Uuid::new_v4().to_string(),
            generation: 0,
            entries: BTreeMap::new(),
            offers: BTreeMap::new(),
            completed: BTreeMap::new(),
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
        self.offers.retain(|_, offer| now < offer.expires);
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
                short_tail_contiguous: false,
                pending: None,
            },
        );
        Ok(lease)
    }
    fn entry_current(
        &self,
        profile: &str,
        c: &PlaybackControl,
        now: Instant,
    ) -> Result<&Entry, Failure> {
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
    fn entry(&self, profile: &str, c: &PlaybackControl, now: Instant) -> Result<&Entry, Failure> {
        let entry = self.entry_current(profile, c, now)?;
        if entry.pending.is_some() {
            return Err("TRANSFER_PENDING");
        }
        Ok(entry)
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
        self.validate_progress_with_pending(p, event, now, false)
    }
    fn validate_progress_with_pending(
        &self,
        p: &str,
        event: &Progress,
        now: Instant,
        allow_pending: bool,
    ) -> Result<Duration, Failure> {
        let e = if allow_pending {
            self.entry_current(p, &control(event), now)?
        } else {
            self.entry(p, &control(event), now)?
        };
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
            e.short_tail_contiguous = short_tail_continues(e, &event, now);
            e.sequence = event.sequence;
            e.sample = Some((event, now));
        }
    }

    fn completed_short_tail(&self, p: &str, event: &Progress, now: Instant) -> bool {
        event.position_ms == event.duration_ms
            && self
                .entries
                .get(p)
                .is_some_and(|e| short_tail_continues(e, event, now))
    }

    fn offer(
        &mut self,
        profile: &str,
        target: AcquirePlayback,
        stream: StreamLease,
        now: Instant,
    ) -> Result<TransferOffer, Failure> {
        self.offers.retain(|_, offer| now < offer.expires);
        let active = self.entries.get(profile).ok_or("LEASE_INVALID")?;
        if now >= active.deadline || active.pending.is_some() {
            return Err("PLAYBACK_ACTIVE");
        }
        if self.offers.len() >= MAX_TRANSFER_TOKENS {
            return Err("TRANSFER_CAPACITY");
        }
        let token = uuid::Uuid::new_v4().to_string();
        self.offers.insert(
            token.clone(),
            Offer {
                profile: profile.into(),
                generation: active.lease.generation,
                target: target.clone(),
                stream,
                expires: now + TRANSFER_OFFER_TIME,
            },
        );
        Ok(TransferOffer {
            token,
            active: active.lease.clone(),
            expires_in_ms: TRANSFER_OFFER_TIME.as_millis() as u64,
        })
    }

    fn confirm(
        &mut self,
        profile: &str,
        token: &str,
        now: Instant,
    ) -> Result<(TransferStatus, PlaybackControl), Failure> {
        let offer = self.offers.get(token).cloned().ok_or("TRANSFER_INVALID")?;
        if offer.profile != profile || now >= offer.expires {
            return Err("TRANSFER_INVALID");
        }
        self.offers.remove(token);
        let entry = self
            .entries
            .get_mut(&offer.profile)
            .ok_or("LEASE_INVALID")?;
        if now >= entry.deadline
            || entry.lease.generation != offer.generation
            || entry.pending.is_some()
        {
            return Err("TRANSFER_INVALID");
        }
        let deadline = entry.deadline;
        entry.lease.transferring = true;
        entry.lease.remaining_ms = deadline.saturating_duration_since(now).as_millis() as u64;
        entry.pending = Some(PendingTransfer {
            token: token.into(),
            target: offer.target,
            stream: offer.stream,
            deadline,
            acknowledged: false,
        });
        self.offers
            .retain(|_, o| o.profile != offer.profile || o.generation != offer.generation);
        let control = PlaybackControl {
            session_id: entry.lease.session_id.clone(),
            generation: entry.lease.generation,
            media_id: entry.lease.media_id.clone(),
            media_revision: entry.lease.media_revision,
        };
        Ok((
            TransferStatus {
                token: token.into(),
                phase: TransferPhase::AwaitingAck,
                remaining_ms: deadline.saturating_duration_since(now).as_millis() as u64,
                lease: None,
                personal: None,
            },
            control,
        ))
    }

    fn ack(&mut self, profile: &str, ack: &TransferAck, now: Instant) -> Result<Progress, Failure> {
        let entry = self.entry_current(profile, &ack.lease, now)?;
        let pending = entry.pending.as_ref().ok_or("TRANSFER_INVALID")?;
        if pending.token != ack.token || pending.acknowledged || now >= pending.deadline {
            return Err("TRANSFER_INVALID");
        }
        // The final ACK preserves the normal sequence/media/range validation.
        self.validate_progress_with_pending(profile, &ack.progress, now, true)?;
        let pending = self
            .entries
            .get_mut(profile)
            .and_then(|e| e.pending.as_mut())
            .expect("checked pending");
        pending.acknowledged = true;
        Ok(ack.progress.clone())
    }

    fn abort_ack(&mut self, token: &str) -> Result<(), Failure> {
        let pending = self
            .entries
            .values_mut()
            .find_map(|entry| entry.pending.as_mut().filter(|p| p.token == token))
            .ok_or("TRANSFER_INVALID")?;
        if !pending.acknowledged {
            return Err("TRANSFER_INVALID");
        }
        pending.acknowledged = false;
        Ok(())
    }

    fn activate(&mut self, token: &str, now: Instant) -> Result<Lease, Failure> {
        let profile = self
            .entries
            .iter()
            .find_map(|(profile, entry)| {
                entry
                    .pending
                    .as_ref()
                    .filter(|p| p.token == token)
                    .map(|_| profile.clone())
            })
            .ok_or("TRANSFER_INVALID")?;
        let entry = self
            .entries
            .get(&profile)
            .expect("located pending transfer");
        let pending = entry.pending.as_ref().expect("located pending transfer");
        if !pending.acknowledged && now < pending.deadline {
            return Err("TRANSFER_PENDING");
        }
        let pending = self
            .entries
            .remove(&profile)
            .and_then(|e| e.pending)
            .expect("located pending transfer");
        // The predecessor may have expired, but the successor is nevertheless
        // activated exactly once; consumers need not infer that from a timeout.
        let phase = TransferPhase::Activated;
        let lease = self.acquire(profile, pending.target, pending.stream, now)?;
        self.completed.insert(
            token.into(),
            TransferStatus {
                token: token.into(),
                phase,
                remaining_ms: lease.remaining_ms,
                lease: Some(lease.clone()),
                personal: None,
            },
        );
        while self.completed.len() > MAX_TRANSFER_TOKENS {
            let first = self.completed.keys().next().cloned().expect("nonempty");
            self.completed.remove(&first);
        }
        Ok(lease)
    }

    fn status(
        &mut self,
        profile: &str,
        token: &str,
        now: Instant,
    ) -> Result<TransferStatus, Failure> {
        if let Some(status) = self.completed.get(token) {
            if status
                .lease
                .as_ref()
                .is_none_or(|lease| lease.profile_id != profile)
            {
                return Err("TRANSFER_INVALID");
            }
            return Ok(status.clone());
        }
        let pending = self
            .entries
            .get(profile)
            .and_then(|entry| entry.pending.as_ref().filter(|p| p.token == token).cloned())
            .ok_or("TRANSFER_INVALID")?;
        if now >= pending.deadline {
            self.activate(token, now)?;
            return self.completed.get(token).cloned().ok_or("TRANSFER_INVALID");
        }
        Ok(TransferStatus {
            token: token.into(),
            phase: TransferPhase::AwaitingAck,
            remaining_ms: pending.deadline.saturating_duration_since(now).as_millis() as u64,
            lease: None,
            personal: None,
        })
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
            "PLAYBACK_ACTIVE" | "LEASE_INVALID" | "TRANSFER_PENDING" | "TRANSFER_INVALID" => {
                StatusCode::CONFLICT
            }
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
                    && e.pending.is_none()
                    && Instant::now() < e.deadline
            })
            .map(|e| e.stream.clone())
            .ok_or(failure("LEASE_INVALID"))
    }
    pub(crate) fn stream_lease_for_profile(
        &self,
        profile: &str,
        session: &str,
        generation: u64,
        media: &str,
    ) -> Result<StreamLease, ApiError> {
        let b = self.book.lock().map_err(|_| failure("MEDIA_BUSY"))?;
        b.entries
            .get(profile)
            .filter(|e| {
                e.lease.session_id == session
                    && e.lease.generation == generation
                    && e.lease.media_id == media
                    && e.pending.is_none()
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
    /// `target` and `stream` must already have passed the same indexed-media
    /// preflight as acquisition; this keeps a proposal bound to that exact file version.
    #[allow(dead_code)]
    pub(crate) fn propose_transfer(
        &self,
        profile: &str,
        target: AcquirePlayback,
        stream: StreamLease,
    ) -> Result<TransferOffer, ApiError> {
        self.book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .offer(profile, target, stream, Instant::now())
            .map_err(failure)
    }
    #[allow(dead_code)]
    pub(crate) fn confirm_transfer(
        &self,
        profile: &str,
        token: &str,
    ) -> Result<TransferStatus, ApiError> {
        let (status, old) = self
            .book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .confirm(profile, token, Instant::now())
            .map_err(failure)?;
        let _ = self.events.send(PauseEvent {
            profile: profile.into(),
            session: old.session_id,
            generation: old.generation,
            token: token.into(),
            remaining_ms: status.remaining_ms,
        });
        Ok(status)
    }
    #[allow(dead_code)]
    pub(crate) fn transfer_status(
        &self,
        profile: &str,
        token: &str,
    ) -> Result<TransferStatus, ApiError> {
        self.book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .status(profile, token, Instant::now())
            .map_err(failure)
    }
    /// The caller persists the returned final event in the serialized actor,
    /// then invokes `activate_transfer` only after that persistence succeeds.
    #[allow(dead_code)]
    pub(crate) fn acknowledge_transfer(
        &self,
        profile: &str,
        ack: &TransferAck,
    ) -> Result<Progress, ApiError> {
        self.book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .ack(profile, ack, Instant::now())
            .map_err(failure)
    }
    /// Undo the ACK reservation when actor persistence fails, before returning
    /// the failure to the client so that the one allowed final event can retry.
    #[allow(dead_code)]
    pub(crate) fn abort_transfer_ack(&self, token: &str) -> Result<(), ApiError> {
        self.book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .abort_ack(token)
            .map_err(failure)
    }
    #[allow(dead_code)]
    pub(crate) fn activate_transfer(&self, token: &str) -> Result<Lease, ApiError> {
        self.book
            .lock()
            .map_err(|_| failure("MEDIA_BUSY"))?
            .activate(token, Instant::now())
            .map_err(failure)
    }
    fn event_receiver(
        &self,
        profile: &str,
        session: &str,
        generation: u64,
    ) -> Result<
        (
            broadcast::Receiver<PauseEvent>,
            tokio::sync::OwnedSemaphorePermit,
            Option<PauseEvent>,
        ),
        ApiError,
    > {
        let b = self.book.lock().map_err(|_| failure("MEDIA_BUSY"))?;
        let control = PlaybackControl {
            session_id: session.into(),
            generation,
            media_id: String::new(),
            media_revision: 0,
        };
        let entry = b.entries.get(profile).ok_or(failure("LEASE_INVALID"))?;
        if entry.lease.session_id != control.session_id
            || entry.lease.generation != control.generation
            || Instant::now() >= entry.deadline
        {
            return Err(failure("LEASE_INVALID"));
        }
        let snapshot = entry.pending.as_ref().map(|pending| PauseEvent {
            profile: profile.into(),
            session: session.into(),
            generation,
            token: pending.token.clone(),
            // Do not replay the confirmation's original value after a client
            // reconnect: the deadline is monotonic and intentionally frozen.
            remaining_ms: pending
                .deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u64,
        });
        drop(b);
        let permit = self
            .event_subscribers
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure("MEDIA_BUSY"))?;
        Ok((self.events.subscribe(), permit, snapshot))
    }
    fn event_alive(&self, profile: &str, session: &str, generation: u64) -> bool {
        self.book
            .lock()
            .ok()
            .and_then(|b| {
                b.entries.get(profile).map(|entry| {
                    entry.lease.session_id == session
                        && entry.lease.generation == generation
                        && Instant::now() < entry.deadline
                })
            })
            .unwrap_or(false)
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/profiles/{profile}/playback/acquire", post(acquire))
        .route(
            "/profiles/{profile}/playback/transfer/propose",
            post(propose_transfer),
        )
        .route(
            "/profiles/{profile}/playback/transfer/confirm",
            post(confirm_transfer),
        )
        .route(
            "/profiles/{profile}/playback/transfer/status",
            get(transfer_status),
        )
        .route(
            "/profiles/{profile}/playback/transfer/ack",
            post(transfer_ack),
        )
        .route("/profiles/{profile}/playback/events", get(events))
        .route("/profiles/{profile}/playback/heartbeat", post(heartbeat))
        .route("/profiles/{profile}/playback/progress", post(progress))
        .route("/profiles/{profile}/playback/release", post(release))
        .route("/profiles/{profile}/playback/speed", post(save_speed))
        .route("/profiles/{profile}/media/{media}/state", get(personal))
        .route("/media/{media}/stream", get(crate::media_stream::stream))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenQuery {
    token: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EventsQuery {
    session: String,
    generation: u64,
}

async fn propose_transfer(
    State(s): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<chill_domain::TransferProposal>,
) -> Result<Json<TransferOffer>, ApiError> {
    let target = body.target;
    if uuid::Uuid::parse_str(&profile).is_err()
        || uuid::Uuid::parse_str(&target.media_id).is_err()
        || uuid::Uuid::parse_str(&target.device_id).is_err()
    {
        return Err(failure("PLAYBACK_REQUEST_INVALID"));
    }
    let p = profile.clone();
    let media = target.media_id.clone();
    let context = target.context.clone();
    let path = s
        .database
        .call(move |store| {
            store.validate_playback_context(&p, &media, &context)?;
            if !store.media(&p, &media)?.available {
                return Ok(None);
            }
            store.media_path(&media).map(Some)
        })
        .await?
        .ok_or(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "MEDIA_UNAVAILABLE",
        ))?;
    let settings = s.library.lock().map_err(|_| failure("MEDIA_BUSY"))?.clone();
    let root = settings
        .video_root
        .clone()
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
    let offer = s
        .database
        .call(move |store| {
            if store.media_path(&target.media_id)? != path
                || !store.media(&profile, &target.media_id)?.available
            {
                return Ok(Err("MEDIA_CHANGED"));
            }
            let current = library.lock().map_err(|_| "INVALID_OPERATION")?;
            if current.revision != settings.revision {
                return Ok(Err("MEDIA_CHANGED"));
            }
            Ok(service
                .propose_transfer(
                    &profile,
                    target.clone(),
                    StreamLease {
                        session: String::new(),
                        generation: 0,
                        media: target.media_id.clone(),
                        root,
                        path,
                        root_revision: settings.revision,
                        version,
                    },
                )
                .map_err(|e| e.1))
        })
        .await?;
    Ok(Json(offer.map_err(failure)?))
}

async fn confirm_transfer(
    State(s): State<AppState>,
    Path(profile): Path<String>,
    Json(body): Json<chill_domain::TransferConfirmation>,
) -> Result<Json<TransferStatus>, ApiError> {
    Ok(Json(s.playback.confirm_transfer(&profile, &body.token)?))
}

async fn transfer_status(
    State(s): State<AppState>,
    Path(profile): Path<String>,
    Query(query): Query<TokenQuery>,
) -> Result<Json<TransferStatus>, ApiError> {
    let mut status = s.playback.transfer_status(&profile, &query.token)?;
    if let Some(lease) = &status.lease {
        status.personal = Some(
            s.database
                .call({
                    let p = profile.clone();
                    let m = lease.media_id.clone();
                    move |store| store.personal_state(&p, &m)
                })
                .await?,
        );
    }
    Ok(Json(status))
}

async fn transfer_ack(
    State(s): State<AppState>,
    Path(profile): Path<String>,
    Json(ack): Json<TransferAck>,
) -> Result<Json<TransferStatus>, ApiError> {
    let service = s.playback.clone();
    let token = ack.token.clone();
    let p = profile.clone();
    let status = s
        .database
        .call(move |store| {
            let final_event = match service.acknowledge_transfer(&p, &ack) {
                Ok(event) => event,
                Err(error) => return Ok(Err(error.1)),
            };
            if let Err(error) = save_progress(store, &service, &p, &final_event, Instant::now()) {
                let _ = service.abort_transfer_ack(&token);
                return Err(error);
            }
            let lease = service
                .activate_transfer(&token)
                .map_err(|e| e.1.to_string())?;
            let personal = store.personal_state(&p, &lease.media_id)?;
            Ok(Ok(TransferStatus {
                token,
                phase: TransferPhase::Activated,
                remaining_ms: lease.remaining_ms,
                lease: Some(lease),
                personal: Some(personal),
            }))
        })
        .await?;
    Ok(Json(status.map_err(failure)?))
}

async fn events(
    State(s): State<AppState>,
    Path(profile): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError>
{
    let (receiver, permit, snapshot) =
        s.playback
            .event_receiver(&profile, &query.session, query.generation)?;
    let service = s.playback.clone();
    let stream = futures_util::stream::unfold(
        (
            receiver,
            permit,
            snapshot,
            service,
            profile,
            query.session,
            query.generation,
        ),
        |(mut receiver, permit, snapshot, service, profile, session, generation)| async move {
            if !service.event_alive(&profile, &session, generation) {
                return None;
            }
            if let Some(event) = snapshot {
                let item = Ok(Event::default().event("pause").data(
                    serde_json::json!({"token":event.token,"remaining_ms":event.remaining_ms})
                        .to_string(),
                ));
                return Some((
                    item,
                    (
                        receiver, permit, None, service, profile, session, generation,
                    ),
                ));
            }
            let next = match tokio::time::timeout(Duration::from_secs(10), receiver.recv()).await {
                Ok(Ok(event))
                    if event.profile == profile
                        && event.session == session
                        && event.generation == generation =>
                {
                    Some(Ok(Event::default().event("pause").data(
                        serde_json::json!({"token":event.token,"remaining_ms":event.remaining_ms})
                            .to_string(),
                    )))
                }
                Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    Some(Ok(Event::default().comment("keepalive")))
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => None,
                Err(_) => Some(Ok(Event::default().comment("keepalive"))),
            };
            next.map(|item| {
                (
                    item,
                    (
                        receiver, permit, None, service, profile, session, generation,
                    ),
                )
            })
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10))))
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
            if media.category == "serie" {
                let key = store.series_identity(&media.id)?;
                store.set_series_speed(&p, &key, change.speed)?;
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
    {
        return Err(failure("PLAYBACK_REQUEST_INVALID"));
    }
    let p = profile.clone();
    let m = body.media_id.clone();
    let context = body.context.clone();
    let path = s
        .database
        .call(move |store| {
            store.validate_playback_context(&p, &m, &context)?;
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
            let saved = save_progress(store, &service, &p, &e, now)?;
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
                save_progress(store, &service, &p, &e, now)?;
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
    #[test]
    fn live_lease_suspends_fingerprint_until_revoked() {
        let (mut book, now, request, stream) = prepared();
        book.acquire("profile".into(), request, stream, now)
            .unwrap();
        let service = PlaybackService::default();
        *service.book.lock().unwrap() = book;
        assert!(service.fingerprint_should_pause());
        service.revoke_profile("profile");
        assert!(!service.fingerprint_should_pause());
    }
    #[test]
    fn short_tail_requires_complete_continuous_final_window_at_each_speed() {
        for speed in [0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0] {
            for paused_eof in [false, true] {
                let (mut b, t, request, stream) = prepared();
                let lease = b.acquire("p".into(), request, stream, t).unwrap();
                let mut first = event(&lease, 1, 53_000, 0);
                first.duration_ms = 60_000;
                first.speed = speed;
                b.record("p", first.clone(), t);
                let mut middle = first.clone();
                middle.sequence = 2;
                middle.position_ms = 57_000;
                middle.final_elapsed_ms = (3_000.0 / speed) as u64;
                let mid = t + Duration::from_secs_f64(4.0 / speed);
                assert!(b.validate_progress("p", &middle, mid).is_ok());
                assert!(!b.completed_short_tail("p", &middle, mid));
                b.record("p", middle.clone(), mid);
                let mut end = middle;
                end.sequence = 3;
                end.position_ms = 60_000;
                end.playing = !paused_eof;
                end.final_elapsed_ms = if paused_eof {
                    0
                } else {
                    (3_000.0 / speed) as u64
                };
                let now = mid + Duration::from_secs_f64(3.0 / speed);
                assert!(b.validate_progress("p", &end, now).is_ok());
                assert!(
                    b.completed_short_tail("p", &end, now),
                    "speed {speed}, paused EOF {paused_eof}"
                );
            }
        }
    }
    #[test]
    fn short_tail_rejects_seek_gaps_buffered_advances_and_new_lease() {
        for defect in ["seek", "buffer", "jump", "duration", "unanchored"] {
            let (mut b, t, request, stream) = prepared();
            let lease = b.acquire("p".into(), request, stream, t).unwrap();
            let mut start = event(&lease, 1, 53_000, 0);
            start.duration_ms = 60_000;
            if defect != "unanchored" {
                b.record("p", start.clone(), t);
            }
            let mut end = start;
            end.sequence = 2;
            end.position_ms = 60_000;
            end.seek = defect == "seek";
            end.buffering = defect == "buffer";
            if defect == "duration" {
                end.duration_ms = 59_000;
                end.position_ms = 59_000;
            }
            let now = if defect == "jump" {
                t
            } else {
                t + Duration::from_secs(7)
            };
            assert!(!b.completed_short_tail("p", &end, now), "{defect}");
        }
    }
    #[test]
    fn short_tail_pause_can_preserve_coverage_but_never_supply_it() {
        let (mut b, t, request, stream) = prepared();
        let lease = b.acquire("p".into(), request, stream, t).unwrap();
        let mut sample = event(&lease, 1, 54_000, 0);
        sample.duration_ms = 60_000;
        b.record("p", sample.clone(), t);
        sample.sequence += 1;
        sample.position_ms = 57_000;
        b.record("p", sample.clone(), t + Duration::from_secs(3));
        sample.sequence += 1;
        sample.playing = false;
        b.record("p", sample.clone(), t + Duration::from_secs(4));
        sample.sequence += 1;
        sample.playing = true;
        b.record("p", sample.clone(), t + Duration::from_secs(5));
        sample.sequence += 1;
        sample.position_ms = 60_000;
        assert!(b.completed_short_tail("p", &sample, t + Duration::from_secs(8)));
        sample.duration_ms = 100_000;
        sample.position_ms = 100_000;
        assert!(
            !b.completed_short_tail("p", &sample, t + Duration::from_secs(48)),
            "100s keeps the existing ten-real-seconds rule"
        );
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

    #[test]
    fn transfer_freezes_deadline_and_activates_once_after_it() {
        let (mut b, t, request, stream) = prepared();
        let old = b
            .acquire("p".into(), request.clone(), stream.clone(), t)
            .unwrap();
        // A prior heartbeat makes the frozen deadline observably different
        // from the original lease lifetime.
        b.heartbeat("p", &ctrl(&old), t + Duration::from_secs(29))
            .unwrap();
        let offer = b
            .offer("p", request, stream, t + Duration::from_secs(29))
            .unwrap();
        let waiting = b
            .confirm("p", &offer.token, t + Duration::from_secs(29))
            .unwrap();
        assert_eq!(waiting.0.remaining_ms, 30_000);
        assert_eq!(
            b.heartbeat("p", &ctrl(&old), t + Duration::from_secs(30))
                .unwrap_err(),
            "TRANSFER_PENDING"
        );
        let status = b
            .status("p", &offer.token, t + Duration::from_secs(59))
            .unwrap();
        let successor = status
            .lease
            .expect("successor activates at frozen deadline");
        assert_eq!(status.phase, TransferPhase::Activated);
        assert!(successor.generation > old.generation);
        assert!(
            b.entry("p", &ctrl(&old), t + Duration::from_secs(59))
                .is_err()
        );
        assert_eq!(
            b.status("p", &offer.token, t + Duration::from_secs(60))
                .unwrap()
                .lease
                .unwrap()
                .generation,
            successor.generation
        );
    }

    #[test]
    fn transfer_ack_is_single_final_event_and_requires_current_generation() {
        let (mut b, t, request, stream) = prepared();
        let old = b
            .acquire("p".into(), request.clone(), stream.clone(), t)
            .unwrap();
        let offer = b.offer("p", request, stream, t).unwrap();
        b.confirm("p", &offer.token, t).unwrap();
        let ack = TransferAck {
            token: offer.token.clone(),
            lease: ctrl(&old),
            progress: event(&old, 1, 12_345, 0),
        };
        assert_eq!(
            b.ack("p", &ack, t + Duration::from_secs(1))
                .unwrap()
                .position_ms,
            12_345
        );
        assert_eq!(
            b.ack("p", &ack, t + Duration::from_secs(1)).unwrap_err(),
            "TRANSFER_INVALID"
        );
        let successor = b
            .activate(&offer.token, t + Duration::from_secs(1))
            .unwrap();
        assert!(successor.generation > old.generation);
        assert!(b.ack("p", &ack, t + Duration::from_secs(2)).is_err());
    }

    #[test]
    fn offers_are_bounded_and_confirmation_isolated_by_profile() {
        let (mut b, t, request, stream) = prepared();
        let first = b
            .acquire("a".into(), request.clone(), stream.clone(), t)
            .unwrap();
        let second = b
            .acquire("b".into(), request.clone(), stream.clone(), t)
            .unwrap();
        let a = b.offer("a", request.clone(), stream.clone(), t).unwrap();
        let b_offer = b.offer("b", request.clone(), stream.clone(), t).unwrap();
        b.confirm("a", &a.token, t).unwrap();
        // Confirming A removes only offers tied to A's generation.
        assert!(b.confirm("b", &b_offer.token, t).is_ok());
        assert_eq!(
            b.heartbeat("a", &ctrl(&first), t).unwrap_err(),
            "TRANSFER_PENDING"
        );
        assert_eq!(
            b.heartbeat("b", &ctrl(&second), t).unwrap_err(),
            "TRANSFER_PENDING"
        );

        let (mut bounded, at, r, s) = prepared();
        let _ = bounded
            .acquire("p".into(), r.clone(), s.clone(), at)
            .unwrap();
        for _ in 0..MAX_TRANSFER_TOKENS {
            bounded.offer("p", r.clone(), s.clone(), at).unwrap();
        }
        assert_eq!(
            bounded.offer("p", r, s, at).unwrap_err(),
            "TRANSFER_CAPACITY"
        );
    }
}
