use crate::{
    native_player::{self, LeaseDeadline, NativeAction, NativeRect, NativeSnapshot},
    offline_downloads::{OfflineContext, OfflineDownloads, OfflineLease},
    player_bridge::{PlayerCommand, error},
};
use chill_client::{
    ClientError,
    playback::{ControlSubscription, Lease, PlaybackControl, ReleasePlayback, TransferAck},
    playback_cache::PlaybackCache,
    ui::{PlaybackConnection, UiError},
    watch_clock::{Observation, WatchClock},
};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::WebviewWindow;

pub struct LocalLaunch {
    pub store: Arc<OfflineDownloads>,
    pub scope: OfflineContext,
    pub copy: OfflineLease,
    pub media_id: String,
    pub cache: PlaybackCache,
}

struct Online {
    connection: PlaybackConnection,
    lease: Lease,
    clock: WatchClock,
    controls: ControlSubscription,
    last_heartbeat: Instant,
}
pub struct LocalPlayer {
    launch: LocalLaunch,
    online: Option<Online>,
    snapshot: NativeSnapshot,
    revision: Option<String>,
    evidence: Evidence,
    last_save: Instant,
    started: bool,
    paused_by_server: bool,
    note: Option<String>,
}

impl LocalPlayer {
    pub async fn open(
        window: &WebviewWindow,
        launch: LocalLaunch,
        rect: NativeRect,
        accepted: Option<(PlaybackConnection, Lease)>,
        sent: Instant,
        position: Option<f64>,
    ) -> Result<Self, UiError> {
        let preferences =
            serde_json::from_value(launch.copy.preferences.clone()).unwrap_or_default();
        let file = launch
            .copy
            .file
            .try_clone()
            .map_err(|_| error("Copie locale inaccessible."))?;
        let deadline = accepted.as_ref().map(|(_, lease)| deadline(lease, sent));
        let opened = native_player::load_local(
            window,
            launch.media_id.clone(),
            file,
            launch.copy.bytes,
            position,
            preferences,
            deadline,
            launch.cache,
        )
        .await;
        match opened {
            Ok(_) => {}
            Err(failure) => {
                if let Some((connection, lease)) = &accepted {
                    let _ = release(connection, lease).await;
                }
                return Err(error(&failure.to_string()));
            }
        };
        let snapshot = match native_player::action(window, NativeAction::Mount { rect }).await {
            Ok(view) => view,
            Err(failure) => {
                let _ = native_player::action(window, NativeAction::Close).await;
                if let Some((connection, lease)) = &accepted {
                    let _ = release(connection, lease).await;
                }
                return Err(error(&failure.to_string()));
            }
        };
        let online = accepted.map(|(connection, lease)| {
            let controls = connection
                .transport
                .subscribe_controls(connection.profile_id.clone(), lease.clone());
            Online {
                connection,
                lease,
                controls,
                clock: WatchClock::default(),
                last_heartbeat: Instant::now(),
            }
        });
        let revision = launch
            .copy
            .progress
            .as_ref()
            .and_then(|progress| progress["base_revision"].as_str().map(str::to_owned))
            .or_else(|| launch.copy.progress_revision.clone());
        Ok(Self {
            launch,
            online,
            snapshot,
            revision,
            evidence: Evidence::default(),
            last_save: Instant::now(),
            started: false,
            paused_by_server: false,
            note: None,
        })
    }

    pub fn view(&self) -> Value {
        let external: Vec<_> = self.launch.copy.subtitles.iter().map(|track|
            json!({"id":track.id,"title":track.title,"language":track.language,"unavailable":false,"ambiguous":false})
        ).collect();
        json!({
            "stage":"player","media_id":self.launch.media_id,"title":self.launch.copy.media["title"],
            "native":self.snapshot,"note":self.note,"active":!self.paused_by_server,
            "position":self.snapshot.position,"duration":self.snapshot.duration,
            "context":{"kind":"standalone","id":null},"next":null,"autoplay_seconds":null,
            "external_subtitles":external,
        })
    }

    pub async fn tick(&mut self, window: &WebviewWindow) -> Result<Option<bool>, UiError> {
        let previous = self.snapshot.clone();
        self.snapshot = native_player::action(window, NativeAction::Poll)
            .await
            .map_err(native_failure)?;
        self.snapshot.duration = self.snapshot.duration.or(previous.duration);
        if self.snapshot.ended && self.snapshot.error.is_none() {
            self.snapshot.position = self.snapshot.duration;
        }
        if let Some(online) = self.online.as_mut()
            && let Ok(control_event) = online.controls.receiver.try_recv()
        {
            let obs = observation(&self.snapshot, false);
            self.snapshot = native_player::action(window, NativeAction::Pause { paused: true })
                .await
                .map_err(native_failure)?;
            if let Some(progress) =
                obs.and_then(|obs| online.clock.event(&online.lease, obs, Instant::now()))
            {
                let ack = TransferAck {
                    token: control_event.token,
                    lease: control(&online.lease),
                    progress,
                };
                let _ = online
                    .connection
                    .transport
                    .acknowledge_transfer(&online.connection.profile_id, &ack)
                    .await;
            }
            self.online = None;
            self.paused_by_server = true;
            self.note = Some("Lecture transférée vers un autre appareil.".into());
            return Ok(None);
        }
        if let Some(online) = &mut self.online
            && online.last_heartbeat.elapsed() >= Duration::from_secs(5)
        {
            let sent = Instant::now();
            online.last_heartbeat = sent;
            let response = tokio::time::timeout(
                Duration::from_secs(3),
                online
                    .connection
                    .transport
                    .heartbeat_playback(&online.connection.profile_id, &control(&online.lease)),
            )
            .await;
            match response {
                Ok(Ok(lease)) => {
                    online.lease = lease;
                    let remaining_ms = deadline(&online.lease, sent).remaining.as_millis() as u64;
                    native_player::action(window, NativeAction::LeaseDeadline { remaining_ms })
                        .await
                        .map_err(native_failure)?;
                }
                Ok(Err(ClientError::Api { .. })) => {
                    self.online = None;
                    self.paused_by_server = true;
                    self.note = Some("Le serveur a interrompu la lecture.".into());
                    self.snapshot =
                        native_player::action(window, NativeAction::Pause { paused: true })
                            .await
                            .map_err(native_failure)?;
                }
                _ => {
                    self.online = None;
                    self.note = Some("Lecture hors connexion.".into());
                    self.snapshot = native_player::action(window, NativeAction::LocalWithoutServer)
                        .await
                        .map_err(native_failure)?;
                }
            }
        }
        if self.paused_by_server {
            return Ok(None);
        }
        if !self.started && self.snapshot.loaded && self.snapshot.error.is_none() {
            let requested = self.launch.copy.preferences["subtitle_language"]
                .as_str()
                .unwrap_or("");
            let preferred: Vec<_> = self
                .launch
                .copy
                .subtitles
                .iter()
                .filter(|track| {
                    !requested.is_empty()
                        && track
                            .language
                            .as_deref()
                            .is_some_and(|language| language.starts_with(requested))
                })
                .collect();
            if preferred.len() == 1
                && !self
                    .snapshot
                    .tracks
                    .iter()
                    .any(|track| track.kind == chill_player::TrackKind::Subtitle && track.selected)
            {
                let track = preferred[0];
                let _ = native_player::action(
                    window,
                    NativeAction::AddSubtitle {
                        path: track.path.clone(),
                        title: track.title.clone(),
                        language: track.language.clone(),
                    },
                )
                .await;
            }
            self.snapshot = native_player::action(window, NativeAction::Pause { paused: false })
                .await
                .map_err(native_failure)?;
            self.started = true;
        }
        let completed = self.evidence.sample(&self.snapshot, Instant::now());
        if self.snapshot.ended {
            self.persist(true, completed).await?;
            return Ok(Some(completed));
        }
        if self.last_save.elapsed() >= Duration::from_secs(5) {
            let _ = self.persist(false, false).await;
        }
        Ok(None)
    }

    pub async fn command(
        &mut self,
        window: &WebviewWindow,
        command: PlayerCommand,
    ) -> Result<Value, UiError> {
        let action = match command {
            PlayerCommand::Status => return Ok(self.view()),
            PlayerCommand::Pause => NativeAction::Pause { paused: true },
            PlayerCommand::Resume if !self.paused_by_server => {
                NativeAction::Pause { paused: false }
            }
            PlayerCommand::Mount { rect } => NativeAction::Mount { rect },
            PlayerCommand::Seek { position } if position.is_finite() => {
                self.reset_evidence();
                NativeAction::Seek { seconds: position }
            }
            PlayerCommand::Jump { seconds } => {
                self.reset_evidence();
                NativeAction::Seek {
                    seconds: self.snapshot.position.unwrap_or_default() + f64::from(seconds),
                }
            }
            PlayerCommand::Speed { speed } if chill_domain::valid_speed(speed) => {
                self.reset_evidence();
                NativeAction::Speed { speed }
            }
            PlayerCommand::Volume { volume } => NativeAction::Volume { volume },
            PlayerCommand::Audio { track_id } => NativeAction::Audio { track_id },
            PlayerCommand::Subtitle { track_id } => NativeAction::Subtitle { track_id },
            PlayerCommand::ExternalSubtitle { id } => {
                let subtitle = self
                    .launch
                    .copy
                    .subtitles
                    .iter()
                    .find(|track| track.id == id)
                    .ok_or_else(|| error("Piste locale indisponible."))?;
                NativeAction::AddSubtitle {
                    path: subtitle.path.clone(),
                    title: subtitle.title.clone(),
                    language: subtitle.language.clone(),
                }
            }
            PlayerCommand::Fullscreen { enabled } => {
                let enabled = enabled.unwrap_or(!window.is_fullscreen().unwrap_or(false));
                window
                    .set_fullscreen(enabled)
                    .map_err(|_| error("Plein écran indisponible."))?;
                NativeAction::Fullscreen
            }
            _ => return Err(error("Commande de lecture locale invalide.")),
        };
        self.snapshot = native_player::action(window, action)
            .await
            .map_err(native_failure)?;
        Ok(self.view())
    }
    fn reset_evidence(&mut self) {
        self.evidence = Evidence::default();
        if let Some(online) = &mut self.online {
            online.clock.forget_credit();
        }
    }

    async fn persist(&mut self, final_event: bool, watched: bool) -> Result<(), UiError> {
        let Some(mut obs) = observation(&self.snapshot, final_event) else {
            if watched {
                return Err(error("La fin du visionnage ne peut pas être enregistrée."));
            }
            return Ok(());
        };
        if final_event && self.snapshot.ended && self.snapshot.error.is_none() {
            obs.position_ms = obs.duration_ms;
            obs.playing = true;
        }
        self.last_save = Instant::now();
        let mut pending = self.online.is_none();
        if let Some(online) = &mut self.online
            && let Some(event) = online.clock.event(&online.lease, obs, self.last_save)
        {
            let response = tokio::time::timeout(
                Duration::from_secs(3),
                online
                    .connection
                    .transport
                    .report_progress_revision(&online.connection.profile_id, &event),
            )
            .await;
            if let Ok(Ok((revision, server_watched))) = response {
                if revision.is_some() {
                    self.revision = revision;
                }
                pending = watched && !server_watched;
                online.clock.acknowledge(event, Instant::now());
            } else {
                online.clock.forget_credit();
                // A missing ACK may have been applied remotely. The compare-and-
                // swap on reconnection will retain any newer confirmed state.
                pending = true;
            }
        }
        let store = self.launch.store.clone();
        let scope = self.launch.scope.clone();
        let media = self.launch.media_id.clone();
        let revision = self.revision.clone();
        tokio::task::spawn_blocking(move || {
            store.save_progress(
                &scope,
                &media,
                obs.position_ms,
                obs.duration_ms,
                watched,
                revision,
                pending,
            )
        })
        .await
        .map_err(|_| error("Enregistrement local interrompu."))?
    }

    pub async fn close(
        mut self,
        window: &WebviewWindow,
        completed: Option<bool>,
    ) -> Result<(), UiError> {
        if completed.is_none() && !self.paused_by_server {
            let _ = self.persist(true, false).await;
        }
        let _ = native_player::action(window, NativeAction::Close).await;
        if let Some(online) = self.online.take() {
            let _ = release(&online.connection, &online.lease).await;
        }
        let LocalLaunch {
            store,
            scope,
            media_id,
            copy,
            ..
        } = self.launch;
        drop(copy);
        if completed == Some(true) {
            tokio::task::spawn_blocking(move || store.auto_delete(&scope, &media_id))
                .await
                .map_err(|_| error("Suppression locale interrompue."))??;
        }
        Ok(())
    }
}

fn native_failure(error_value: native_player::NativePlayerError) -> UiError {
    error(&error_value.to_string())
}
fn control(value: &Lease) -> PlaybackControl {
    PlaybackControl {
        session_id: value.session_id.clone(),
        generation: value.generation,
        media_id: value.media_id.clone(),
        media_revision: value.media_revision,
    }
}
async fn release(connection: &PlaybackConnection, lease: &Lease) -> Result<(), ClientError> {
    connection
        .transport
        .release_playback(
            &connection.profile_id,
            &ReleasePlayback {
                lease: control(lease),
                progress: None,
            },
        )
        .await
}
fn deadline(value: &Lease, sent: Instant) -> LeaseDeadline {
    LeaseDeadline {
        remaining: Duration::from_millis(value.remaining_ms)
            .saturating_sub(sent.elapsed())
            .saturating_sub(Duration::from_millis(250)),
    }
}
fn observation(snapshot: &NativeSnapshot, finished: bool) -> Option<Observation> {
    let duration = snapshot.duration.filter(|d| d.is_finite() && *d > 0.)?;
    let position = snapshot
        .position
        .filter(|p| p.is_finite())?
        .clamp(0., duration);
    Some(Observation {
        duration_ms: (duration * 1000.).round() as u64,
        position_ms: (position * 1000.).round() as u64,
        playing: !finished && !snapshot.paused && !snapshot.ended,
        buffering: snapshot.buffering,
        seek: false,
        speed: snapshot.speed,
    })
}
#[derive(Default)]
struct Evidence {
    previous: Option<(Observation, Instant)>,
    tail_motion_ms: f64,
    credit_ms: f64,
    before_tail: bool,
}
impl Evidence {
    fn sample(&mut self, snapshot: &NativeSnapshot, now: Instant) -> bool {
        if snapshot.error.is_some() {
            *self = Self::default();
            return false;
        }
        // libmpv can clear time-pos one poll before emitting its EOF event.
        // Keep the last observation across a missing value, with the usual
        // time and seek limits still applied when the next value arrives.
        let Some(current) = observation(snapshot, false).filter(|_| snapshot.loaded) else {
            return false;
        };
        if !chill_domain::valid_speed(current.speed) {
            *self = Self::default();
            return false;
        }
        let start = chill_domain::final_tenth_start(current.duration_ms);
        if current.playing && !current.buffering && current.position_ms <= start {
            self.before_tail = true;
        }
        if let Some((last, old)) = self.previous {
            let elapsed = now.saturating_duration_since(old).as_secs_f64() * 1000.;
            let distance = current.position_ms as f64 - last.position_ms as f64;
            if last.playing
                && !last.buffering
                && ((current.playing && !current.buffering) || snapshot.ended)
                && last.speed == current.speed
                && current.duration_ms == last.duration_ms
                && elapsed > 0.
                && elapsed <= 2500.
                && distance >= 0.
                && distance <= elapsed * current.speed + 300.
            {
                let tail = current
                    .position_ms
                    .saturating_sub(last.position_ms.max(start)) as f64;
                self.credit_ms += elapsed.min(tail / current.speed);
                if self.before_tail {
                    // A bounded observation measures actual playhead movement.
                    // Per-sample clock rounding must not accumulate as lost coverage.
                    self.tail_motion_ms += tail;
                }
            } else if !(0. ..=300.).contains(&distance) || elapsed > 2500. {
                *self = Self::default();
            }
        }
        self.previous = Some((current, now));
        snapshot.ended
            && if current.duration_ms < 100000 {
                self.before_tail
                    && self.tail_motion_ms
                        >= (current.duration_ms - start).saturating_sub(100).max(1) as f64
            } else {
                self.credit_ms >= 10000.
            }
    }
}
