//! Serialized native-host coordinator. JavaScript never supplies grants, URLs or paths.
use crate::native_player::{self, LeaseDeadline, NativeAction, NativeRect, NativeSnapshot};
use chill_client::{
    Transport,
    playback::{AcquirePlayback, Lease, PlaybackControl, PlaybackInfo, ReleasePlayback},
    ui::{PlaybackConnection, UiError},
    watch_clock::{Observation, WatchClock},
};
use chill_player::{PlaybackStream, TrackKind};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::WebviewWindow;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerCommand {
    Start { media_id: String, rect: NativeRect },
    Status,
    Pause,
    Resume,
    Stop,
    Mount { rect: NativeRect },
    Seek { position: f64 },
    Jump { seconds: i8 },
    Speed { speed: f64 },
    Audio { track_id: i64 },
    Subtitle { track_id: Option<i64> },
    Fullscreen,
}
pub fn error(message: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: "PLAYER_ERROR".into(),
    }
}
fn native_error(e: native_player::NativePlayerError) -> UiError {
    error(&e.to_string())
}
fn control(l: &Lease) -> PlaybackControl {
    PlaybackControl {
        session_id: l.session_id.clone(),
        generation: l.generation,
        media_id: l.media_id.clone(),
        media_revision: l.media_revision,
    }
}
struct Session {
    transport: Transport,
    profile: String,
    device: String,
    info: PlaybackInfo,
    lease: Option<Lease>,
    clock: WatchClock,
    last_save: Instant,
    last_beat: Instant,
    snapshot: NativeSnapshot,
    rect: NativeRect,
    started: bool,
    desired_position: Option<f64>,
    note: Option<String>,
}
#[derive(Default)]
pub struct PlayerHost {
    session: Option<Session>,
    pub rendering: Arc<AtomicBool>,
}
impl PlayerHost {
    pub async fn start(
        &mut self,
        w: &WebviewWindow,
        c: PlaybackConnection,
        media: String,
        rect: NativeRect,
    ) -> Result<Value, UiError> {
        uuid_check(&media)?;
        self.stop(w).await?;
        let info = c.transport.playback_info(&c.profile_id, &media).await?;
        if !info.media.available {
            return Err(error("Vidéo indisponible."));
        }
        let sent = Instant::now();
        let grant = c
            .transport
            .acquire_playback(
                &c.profile_id,
                &AcquirePlayback {
                    media_id: media,
                    device_id: c.device_id.clone(),
                    context: Default::default(),
                },
            )
            .await?;
        let remaining = conservative(grant.lease.remaining_ms, sent);
        let loaded = native_player::load(
            w,
            c.transport.clone(),
            stream(&grant.lease),
            Some(grant.personal.position_ms as f64 / 1000.0),
            Some(info.preferences.audio_language.clone()),
            Some(info.preferences.subtitle_language.clone()),
            info.speed,
            LeaseDeadline { remaining },
        )
        .await;
        let snapshot = match loaded {
            Ok(s) => s,
            Err(e) => {
                let _ = c
                    .transport
                    .release_playback(
                        &c.profile_id,
                        &ReleasePlayback {
                            lease: control(&grant.lease),
                            progress: None,
                        },
                    )
                    .await;
                return Err(native_error(e));
            }
        };
        native_player::action(w, NativeAction::Mount { rect })
            .await
            .map_err(native_error)?;
        self.session = Some(Session {
            transport: c.transport,
            profile: c.profile_id,
            device: c.device_id,
            info,
            lease: Some(grant.lease),
            clock: WatchClock::default(),
            last_save: Instant::now(),
            last_beat: Instant::now(),
            snapshot,
            rect,
            started: false,
            desired_position: None,
            note: None,
        });
        self.view()
    }
    pub fn view(&self) -> Result<Value, UiError> {
        Ok(match &self.session {
            None => json!({"stage":"closed"}),
            Some(s) => {
                json!({"stage":"player","media_id":s.info.media.id,"title":s.info.media.title,"native":s.snapshot,"note":s.note,"position":s.desired_position.or(s.snapshot.position),"duration":s.snapshot.duration,"active":s.lease.is_some()})
            }
        })
    }
    pub async fn tick(&mut self, w: &WebviewWindow) -> Result<(), UiError> {
        let Some(s) = &mut self.session else {
            return Ok(());
        };
        let previous = s.snapshot.clone();
        s.snapshot = native_player::action(w, NativeAction::Poll)
            .await
            .map_err(native_error)?;
        if s.snapshot.duration.is_none() {
            s.snapshot.duration = previous.duration;
        }
        if s.snapshot.ended && s.snapshot.position.is_none() {
            s.snapshot.position = if s.snapshot.error.is_none() {
                s.snapshot.duration
            } else {
                previous.position
            };
        }
        if s.snapshot.error.is_some() {
            self.rendering.store(false, Ordering::Release);
            let _ = native_player::action(w, NativeAction::Pause { paused: true }).await;
            s.snapshot.paused = true;
            if let Some(l) = s.lease.take() {
                let _ = s
                    .transport
                    .release_playback(
                        &s.profile,
                        &ReleasePlayback {
                            lease: control(&l),
                            progress: None,
                        },
                    )
                    .await;
            }
            s.clock.forget_credit();
            return Ok(());
        }
        if s.snapshot.ended {
            self.rendering.store(false, Ordering::Release);
            if s.lease.is_some() {
                report(s, !previous.paused, false).await?;
                release(s).await?;
            }
            return Ok(());
        }
        if !s.started && s.snapshot.loaded && s.lease.is_some() {
            renew(s, w).await?;
            s.snapshot = native_player::action(w, NativeAction::Pause { paused: false })
                .await
                .map_err(native_error)?;
            s.started = true;
            report(s, true, true).await?; // Anchor only, initial resume is a seek, never credit.
            report(s, true, false).await?;
        }
        if let Some(target)=s.desired_position {
            if s.lease.is_some() && s.snapshot.position.is_some_and(|p|(p-target).abs()<0.5) && !s.snapshot.buffering {
                s.desired_position=None;s.clock.forget_credit();
                report(s,!s.snapshot.paused,true).await?;
                report(s,!s.snapshot.paused,false).await?;
            }
        }
        self.rendering.store(
            s.started && !s.snapshot.paused && !s.snapshot.buffering && !s.snapshot.ended,
            Ordering::Release,
        );
        if s.lease.is_some() && s.last_beat.elapsed() >= Duration::from_secs(5) {
            if let Err(e) = renew(s, w).await {
                let _ = native_player::action(w, NativeAction::Pause { paused: true }).await;
                s.snapshot.paused = true;
                s.note = Some(e.message.clone());
                s.clock.forget_credit();
                self.rendering.store(false, Ordering::Release);
                return Err(e);
            }
        }
        if s.lease.is_some() && s.started && s.last_save.elapsed() >= Duration::from_secs(5) {
            report(s, !s.snapshot.paused, s.desired_position.is_some()).await?;
        }
        Ok(())
    }
    pub async fn command(
        &mut self,
        w: &WebviewWindow,
        cmd: PlayerCommand,
    ) -> Result<Value, UiError> {
        if matches!(cmd, PlayerCommand::Stop) {
            self.stop(w).await?;
            return self.view();
        }
        if matches!(cmd, PlayerCommand::Status) {
            return self.view();
        }
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| error("Aucune vidéo ouverte."))?;
        match cmd {
            PlayerCommand::Pause => {
                let was_playing = !s.snapshot.paused;
                s.snapshot = native_player::action(w, NativeAction::Pause { paused: true })
                    .await
                    .map_err(native_error)?;
                self.rendering.store(false, Ordering::Release);
                if s.lease.is_some() {
                    report(s, was_playing, false).await?;
                    release(s).await?;
                }
            }
            PlayerCommand::Resume => {
                if s.lease.is_none() {
                    let sent = Instant::now();
                    let g = s
                        .transport
                        .acquire_playback(
                            &s.profile,
                            &AcquirePlayback {
                                media_id: s.info.media.id.clone(),
                                device_id: s.device.clone(),
                                context: Default::default(),
                            },
                        )
                        .await?;
                    s.clock = WatchClock::default();
                    s.snapshot = native_player::load(
                        w,
                        s.transport.clone(),
                        stream(&g.lease),
                        Some(g.personal.position_ms as f64 / 1000.0),
                        Some(s.info.preferences.audio_language.clone()),
                        Some(s.info.preferences.subtitle_language.clone()),
                        s.snapshot.speed,
                        LeaseDeadline {
                            remaining: conservative(g.lease.remaining_ms, sent),
                        },
                    )
                    .await
                    .map_err(native_error)?;
                    s.lease = Some(g.lease);
                    s.started = false;
                    s.last_beat = Instant::now();
                    s.desired_position = None;
                    native_player::action(w, NativeAction::Mount { rect: s.rect })
                        .await
                        .map_err(native_error)?;
                } else {
                    renew(s, w).await?;
                    s.snapshot = native_player::action(w, NativeAction::Pause { paused: false })
                        .await
                        .map_err(native_error)?;
                }
            }
            PlayerCommand::Mount { rect } => {
                s.snapshot = native_player::action(w, NativeAction::Mount { rect })
                    .await
                    .map_err(native_error)?;
                s.rect = rect;
            }
            PlayerCommand::Seek { position } => seek(s, w, position).await?,
            PlayerCommand::Jump { seconds } => {
                if ![-10, 30].contains(&seconds) {
                    return Err(error("Saut invalide."));
                }
                let position =
                    s.desired_position.or(s.snapshot.position).unwrap_or(0.0) + seconds as f64;
                seek(s, w, position).await?;
            }
            PlayerCommand::Speed { speed } => {
                if !chill_client::playback::valid_speed_value(speed) {
                    return Err(error("Vitesse invalide."));
                }
                s.clock.forget_credit();
                if let Some(l) = &s.lease {
                    s.transport
                        .save_playback_speed(&s.profile, &control(l), speed)
                        .await?;
                }
                s.snapshot = native_player::action(w, NativeAction::Speed { speed })
                    .await
                    .map_err(native_error)?;
                if s.lease.is_some() {
                    report(s, !s.snapshot.paused, true).await?;
                }
            }
            PlayerCommand::Audio { track_id } => {
                if !s
                    .snapshot
                    .tracks
                    .iter()
                    .any(|t| t.id == track_id && t.kind == TrackKind::Audio)
                {
                    return Err(error("Piste audio absente."));
                }
                s.snapshot = native_player::action(w, NativeAction::Audio { track_id })
                    .await
                    .map_err(native_error)?;
            }
            PlayerCommand::Subtitle { track_id } => {
                if track_id.is_some_and(|id| {
                    !s.snapshot
                        .tracks
                        .iter()
                        .any(|t| t.id == id && t.kind == TrackKind::Subtitle)
                }) {
                    return Err(error("Sous-titre absent."));
                }
                s.snapshot = native_player::action(w, NativeAction::Subtitle { track_id })
                    .await
                    .map_err(native_error)?;
            }
            PlayerCommand::Fullscreen => {
                w.set_fullscreen(
                    !w.is_fullscreen()
                        .map_err(|_| error("Plein écran indisponible."))?,
                )
                .map_err(|_| error("Plein écran indisponible."))?;
            }
            _ => return Err(error("Commande de lecture invalide.")),
        }
        self.view()
    }
    pub async fn stop(&mut self, w: &WebviewWindow) -> Result<(), UiError> {
        self.rendering.store(false, Ordering::Release);
        if let Some(mut s) = self.session.take() {
            let snapshot = native_player::action(w, NativeAction::Poll)
                .await
                .map_err(native_error)?;
            let playing = !snapshot.paused;
            s.snapshot = snapshot;
            let _ = native_player::action(w, NativeAction::Close).await;
            if s.lease.is_some() {
                let _ = report(&mut s, playing, false).await;
                let _ = release(&mut s).await;
            }
        }
        Ok(())
    }
}
fn stream(l: &Lease) -> PlaybackStream {
    PlaybackStream {
        media_id: l.media_id.clone(),
        session_id: l.session_id.clone(),
        generation: l.generation,
    }
}
fn conservative(ms: u64, sent: Instant) -> Duration {
    Duration::from_millis(ms)
        .saturating_sub(sent.elapsed())
        .saturating_sub(Duration::from_millis(250))
}
fn uuid_check(s: &str) -> Result<(), UiError> {
    if s.len() != 36 || s.bytes().any(|b| !b.is_ascii_hexdigit() && b != b'-') {
        Err(error("Identifiant vidéo invalide."))
    } else {
        Ok(())
    }
}
async fn renew(s: &mut Session, w: &WebviewWindow) -> Result<(), UiError> {
    let l = s
        .lease
        .as_ref()
        .ok_or_else(|| error("Lecture sans bail."))?;
    let sent = Instant::now();
    let fresh = s
        .transport
        .heartbeat_playback(&s.profile, &control(l))
        .await?;
    let remaining = conservative(fresh.remaining_ms, sent).as_millis() as u64;
    native_player::action(
        w,
        NativeAction::LeaseDeadline {
            remaining_ms: remaining,
        },
    )
    .await
    .map_err(native_error)?;
    s.lease = Some(fresh);
    s.last_beat = Instant::now();
    Ok(())
}
async fn report(s: &mut Session, playing: bool, seek: bool) -> Result<(), UiError> {
    let Some(l) = s.lease.as_ref() else {
        return Ok(());
    };
    let obs = Observation {
        position_ms: (s
            .desired_position
            .or(s.snapshot.position)
            .unwrap_or_default()
            .max(0.0)
            * 1000.0)
            .round() as u64,
        duration_ms: (s.snapshot.duration.unwrap_or_default().max(0.0) * 1000.0).round() as u64,
        speed: s.snapshot.speed,
        playing,
        buffering: s.snapshot.buffering,
        seek,
    };
    if let Some(event) = s.clock.event(l, obs, Instant::now()) {
        if let Err(error) = s.transport.report_progress(&s.profile, &event).await {
            s.clock.forget_credit();
            return Err(error.into());
        }
        s.clock.acknowledge(event, Instant::now());
        s.last_save = Instant::now();
    }
    Ok(())
}
async fn release(s: &mut Session) -> Result<(), UiError> {
    if let Some(l) = s.lease.take() {
        s.clock.forget_credit();
        s.transport
            .release_playback(
                &s.profile,
                &ReleasePlayback {
                    lease: control(&l),
                    progress: None,
                },
            )
            .await?;
    }
    Ok(())
}
async fn seek(s: &mut Session, w: &WebviewWindow, position: f64) -> Result<(), UiError> {
    if !position.is_finite() {
        return Err(error("Position invalide."));
    }
    let duration = s
        .snapshot
        .duration
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| error("Durée encore inconnue."))?;
    let target = position.max(0.0).min(duration);
    s.clock.forget_credit();
    s.desired_position = Some(target);
    if s.lease.is_none() {
        let g = s
            .transport
            .acquire_playback(
                &s.profile,
                &AcquirePlayback {
                    media_id: s.info.media.id.clone(),
                    device_id: s.device.clone(),
                    context: Default::default(),
                },
            )
            .await?;
        s.clock = WatchClock::default();
        s.lease = Some(g.lease);
        report(s, false, true).await?;
        release(s).await?;
    } else {
        s.snapshot = native_player::action(w, NativeAction::Seek { seconds: target })
            .await
            .map_err(native_error)?;
        report(s, !s.snapshot.paused, true).await?;
    }
    Ok(())
}
