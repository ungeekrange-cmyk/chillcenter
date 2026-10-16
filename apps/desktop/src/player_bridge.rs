//! Serialized native-host coordinator. JavaScript never supplies grants, URLs or paths.
use crate::native_player::{self, LeaseDeadline, NativeAction, NativeRect, NativeSnapshot};
use chill_client::{
    Transport,
    playback::{
        AcquirePlayback, ControlSubscription, ExternalSubtitle, Lease, NextVideo, PlaybackContext,
        PlaybackControl, PlaybackGrant, PlaybackInfo, ReleasePlayback, TransferAck, TransferOffer,
        TransferPhase, TransferStatus,
    },
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
    Start {
        media_id: String,
        rect: NativeRect,
        #[serde(default)]
        context: Option<PlaybackContext>,
    },
    Status,
    ConfirmTransfer,
    RefuseTransfer,
    Next {
        expected_id: String,
    },
    Pause,
    Resume,
    Stop,
    Mount {
        rect: NativeRect,
    },
    Seek {
        position: f64,
    },
    Jump {
        seconds: i8,
    },
    Speed {
        speed: f64,
    },
    Audio {
        track_id: i64,
    },
    Subtitle {
        track_id: Option<i64>,
    },
    ExternalSubtitle {
        id: String,
    },
    Fullscreen {
        #[serde(default)]
        enabled: Option<bool>,
    },
    Volume {
        volume: u8,
    },
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
    cache: chill_client::playback_cache::PlaybackCache,
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
    context: PlaybackContext,
    next: Option<NextVideo>,
    external: Vec<ExternalSubtitle>,
    selected_external: Option<String>,
    audio_override: Option<i64>,
    subtitle_override: Option<Option<i64>>,
    subtitle_cache: Vec<(String, std::path::PathBuf)>,
    cache_dir: std::path::PathBuf,
    controls: Option<ControlSubscription>,
    external_task: Option<tokio::task::JoinHandle<Result<DiscoveredSubtitles, UiError>>>,
}
struct DiscoveredSubtitles {
    external: Vec<ExternalSubtitle>,
    selected_before: Option<String>,
    preferred: Option<(ExternalSubtitle, Result<Vec<u8>, UiError>)>,
}
struct PendingTransfer {
    connection: PlaybackConnection,
    info: PlaybackInfo,
    rect: NativeRect,
    context: PlaybackContext,
    offer: TransferOffer,
    confirmed: bool,
    last_poll: Instant,
    remaining_ms: u64,
}
#[derive(Default)]
pub struct PlayerHost {
    session: Option<Session>,
    pending: Option<PendingTransfer>,
    last_error: Option<String>,
    pub rendering: Arc<AtomicBool>,
    pub cache_dir: std::path::PathBuf,
}
impl PlayerHost {
    pub async fn start(
        &mut self,
        w: &WebviewWindow,
        c: PlaybackConnection,
        media: String,
        rect: NativeRect,
        context: Option<PlaybackContext>,
    ) -> Result<Value, UiError> {
        uuid_check(&media)?;
        self.stop_session(w, false).await?;
        let info = c.transport.playback_info(&c.profile_id, &media).await?;
        if !info.media.available {
            return Err(error("Vidéo indisponible."));
        }
        let context = context.unwrap_or_else(|| {
            if info.media.category == "serie" {
                PlaybackContext {
                    kind: "series".into(),
                    id: Some(
                        info.media
                            .tmdb_id
                            .map(|id| format!("tmdb:tv:{id}"))
                            .unwrap_or_else(|| format!("media:{}", info.media.id)),
                    ),
                }
            } else {
                Default::default()
            }
        });
        let sent = Instant::now();
        let acquired = c
            .transport
            .acquire_playback(
                &c.profile_id,
                &AcquirePlayback {
                    media_id: media,
                    device_id: c.device_id.clone(),
                    context: context.clone(),
                },
            )
            .await;
        let grant = match acquired {
            Ok(grant) => grant,
            Err(chill_client::ClientError::Api { code, .. }) if code == "PLAYBACK_ACTIVE" => {
                let offer = c
                    .transport
                    .propose_transfer(
                        &c.profile_id,
                        &AcquirePlayback {
                            media_id: info.media.id.clone(),
                            device_id: c.device_id.clone(),
                            context: context.clone(),
                        },
                    )
                    .await?;
                self.pending = Some(PendingTransfer {
                    connection: c,
                    info,
                    rect,
                    context,
                    offer,
                    confirmed: false,
                    last_poll: Instant::now(),
                    remaining_ms: 0,
                });
                return self.view();
            }
            Err(e) => return Err(e.into()),
        };
        self.install(w, c, info, rect, context, grant, sent).await
    }
    // Captured transfer/grant inputs stay explicit; no implicit UI/global state.
    #[allow(clippy::too_many_arguments)]
    async fn install(
        &mut self,
        w: &WebviewWindow,
        c: PlaybackConnection,
        info: PlaybackInfo,
        rect: NativeRect,
        context: PlaybackContext,
        grant: PlaybackGrant,
        sent: Instant,
    ) -> Result<Value, UiError> {
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
            c.cache,
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
        if let Err(e) = native_player::action(w, NativeAction::Mount { rect }).await {
            let _ = native_player::action(w, NativeAction::Close).await;
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
        let controls = Some(
            c.transport
                .subscribe_controls(c.profile_id.clone(), grant.lease.clone()),
        );
        self.session = Some(Session {
            transport: c.transport,
            cache: c.cache,
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
            context,
            next: None,
            external: Vec::new(),
            selected_external: None,
            audio_override: None,
            subtitle_override: None,
            subtitle_cache: Vec::new(),
            cache_dir: self.cache_dir.clone(),
            controls,
            external_task: None,
        });
        self.view()
    }
    pub fn view(&self) -> Result<Value, UiError> {
        if let Some(p) = &self.pending {
            return Ok(
                json!({"stage":"transfer","title":p.info.media.title,"confirmed":p.confirmed,"remaining_ms":p.remaining_ms,"note":"Ce profil lit déjà une vidéo sur un autre appareil. Confirmez pour y interrompre la lecture et la reprendre ici."}),
            );
        }
        Ok(match &self.session {
            None => json!({"stage":"closed","error":self.last_error}),
            Some(s) => {
                json!({"stage":"player","media_id":s.info.media.id,"title":s.info.media.title,"native":s.snapshot,"note":s.note,"position":s.desired_position.or(s.snapshot.position),"duration":s.snapshot.duration,"active":s.lease.is_some(),"context":s.context,"next":s.next,"autoplay_seconds":s.info.preferences.autoplay_seconds,"external_subtitles":s.external})
            }
        })
    }
    pub async fn tick(&mut self, w: &WebviewWindow) -> Result<(), UiError> {
        if let Some(p) = self.pending.as_mut() {
            if p.confirmed && p.last_poll.elapsed() >= Duration::from_secs(1) {
                p.last_poll = Instant::now();
                let sent = Instant::now();
                match p
                    .connection
                    .transport
                    .transfer_status(&p.connection.profile_id, &p.offer.token)
                    .await
                {
                    Ok(status) => {
                        self.finish_transfer(w, status, sent).await?;
                    }
                    Err(e) => {
                        self.last_error = Some(UiError::from(e).message);
                        self.pending = None;
                    }
                }
            }
            return Ok(());
        }
        let Some(s) = &mut self.session else {
            return Ok(());
        };
        if let Some(event) = s.controls.as_mut().and_then(|c| c.receiver.try_recv().ok()) {
            if let Some(task) = s.external_task.take() {
                task.abort();
            }
            self.rendering.store(false, Ordering::Release);
            let playing = !s.snapshot.paused;
            let previous = s.snapshot.clone();
            if let Ok(mut snapshot) =
                native_player::action(w, NativeAction::Pause { paused: true }).await
            {
                if snapshot.position.is_none() {
                    snapshot.position = previous.position;
                }
                if snapshot.duration.is_none() {
                    snapshot.duration = previous.duration;
                }
                s.snapshot = snapshot;
                s.controls = None;
                if let Some(lease) = s.lease.take() {
                    let obs = observation(s, playing, false);
                    if let Some(progress) = s.clock.event(&lease, obs, Instant::now()) {
                        let ack = TransferAck {
                            token: event.token,
                            lease: control(&lease),
                            progress,
                        };
                        if s.transport
                            .acknowledge_transfer(&s.profile, &ack)
                            .await
                            .is_err()
                        {
                            s.note=Some("Lecture interrompue pour transfert ; synchronisation finale indisponible.".into());
                        } else {
                            s.note = Some("Lecture transférée vers l'autre appareil.".into());
                        }
                    } else {
                        s.note=Some("Lecture interrompue pour transfert ; l'autre appareil attend l'échéance du bail.".into());
                    }
                }
                s.clock.forget_credit();
                return Ok(());
            }
            return Ok(());
        }
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
            if let Some(l) = &s.lease
                && matches!(s.transport.heartbeat_playback(&s.profile,&control(l)).await,Err(chill_client::ClientError::Api{code,..}) if code=="TRANSFER_PENDING")
            {
                let _ = native_player::action(w, NativeAction::Pause { paused: true }).await;
                s.snapshot.paused = true;
                s.snapshot.position = s.snapshot.position.or(previous.position);
                s.note = Some("Interruption pour transfert en cours…".into());
                return Ok(());
            }
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
            abandon(s);
            return Ok(());
        }
        if s.snapshot.ended {
            self.rendering.store(false, Ordering::Release);
            if s.lease.is_some() {
                let saved = report(s, !previous.paused, false).await;
                let released = release(s).await;
                native_player::action(w, NativeAction::Close)
                    .await
                    .map_err(native_error)?;
                if saved.is_err() || released.is_err() {
                    s.note=Some("Fin de lecture : synchronisation indisponible, aucun enchaînement automatique.".into());
                } else {
                    match s
                        .transport
                        .next_video(&s.profile, &s.info.media.id, &s.context)
                        .await
                    {
                        Ok(next) => s.next = Some(next),
                        Err(_) => {
                            s.note = Some(
                                "Vidéo suivante indisponible : impossible de vérifier la suite."
                                    .into(),
                            )
                        }
                    }
                }
            }
            return Ok(());
        }
        if !s.started && s.snapshot.loaded && s.lease.is_some() {
            renew(s, w).await?;
            if let Some(track_id) = s.audio_override
                && s.snapshot
                    .tracks
                    .iter()
                    .any(|t| t.id == track_id && t.kind == TrackKind::Audio)
            {
                s.snapshot = native_player::action(w, NativeAction::Audio { track_id })
                    .await
                    .map_err(native_error)?;
            }
            if let Some(track_id) = s.subtitle_override
                && (track_id.is_none()
                    || s.snapshot
                        .tracks
                        .iter()
                        .any(|t| Some(t.id) == track_id && t.kind == TrackKind::Subtitle))
            {
                s.snapshot = native_player::action(w, NativeAction::Subtitle { track_id })
                    .await
                    .map_err(native_error)?;
            }
            s.snapshot = native_player::action(w, NativeAction::Pause { paused: false })
                .await
                .map_err(native_error)?;
            s.started = true;
            self.rendering.store(true, Ordering::Release);
            report(s, true, true).await?; // Anchor only, initial resume is a seek, never credit.
            report(s, true, false).await?;
            discover_external(s);
        }
        // Network discovery/download never owns the host mutex or delays the
        // first unpause. Session-bound results are consumed only while its lease
        // remains valid, and a newer manual choice always wins.
        if s.external_task
            .as_ref()
            .is_some_and(|task| task.is_finished())
        {
            let task = s.external_task.take().unwrap();
            match task.await {
                Ok(Ok(found)) if s.lease.is_some() && !s.snapshot.paused => {
                    s.external = found.external;
                    if s.subtitle_override.is_none() && s.selected_external == found.selected_before
                    {
                        if let Some((candidate, bytes)) = found.preferred {
                            let applied = match bytes {
                                Ok(bytes) => {
                                    apply_downloaded_external(s, w, candidate, bytes).await
                                }
                                Err(error) => Err(error),
                            };
                            if applied.is_err() {
                                s.note = Some(
                                    "Sous-titre externe indisponible ; choisissez une autre piste."
                                        .into(),
                                );
                            }
                        } else if matching_external(
                            &s.external,
                            &s.info.preferences.subtitle_language,
                        )
                        .len()
                            > 1
                        {
                            s.note = Some("Plusieurs sous-titres correspondent à votre langue : choisissez la piste souhaitée.".into());
                        }
                    }
                }
                Ok(Err(_)) => {
                    s.note = Some("Impossible de vérifier les sous-titres externes.".into())
                }
                _ => {}
            }
        }
        if let Some(target) = s.desired_position
            && s.lease.is_some()
            && s.snapshot
                .position
                .is_some_and(|p| (p - target).abs() < 0.5)
            && !s.snapshot.buffering
        {
            s.desired_position = None;
            s.clock.forget_credit();
            report(s, !s.snapshot.paused, true).await?;
            report(s, !s.snapshot.paused, false).await?;
        }
        self.rendering.store(
            s.started && !s.snapshot.paused && !s.snapshot.buffering && !s.snapshot.ended,
            Ordering::Release,
        );
        if s.lease.is_some()
            && s.last_beat.elapsed() >= Duration::from_secs(5)
            && let Err(e) = renew(s, w).await
        {
            let _ = native_player::action(w, NativeAction::Pause { paused: true }).await;
            s.snapshot.paused = true;
            s.note = Some(e.message.clone());
            abandon(s);
            self.rendering.store(false, Ordering::Release);
            return Err(e);
        }
        if s.lease.is_some() && s.started && s.last_save.elapsed() >= Duration::from_secs(5) {
            s.last_save = Instant::now();
            if let Err(e) = report(s, !s.snapshot.paused, s.desired_position.is_some()).await {
                s.note = Some("Synchronisation de la reprise momentanément indisponible.".into());
                let _ = native_player::action(w, NativeAction::Pause { paused: true }).await;
                s.snapshot.paused = true;
                abandon(s);
                self.rendering.store(false, Ordering::Release);
                return Err(e);
            }
        }
        Ok(())
    }
    pub async fn command(
        &mut self,
        w: &WebviewWindow,
        cmd: PlayerCommand,
    ) -> Result<Value, UiError> {
        if matches!(cmd, PlayerCommand::RefuseTransfer) {
            if self.pending.as_ref().is_some_and(|p| p.confirmed) {
                return Err(error("Le transfert est déjà confirmé."));
            }
            self.pending = None;
            w.set_fullscreen(false)
                .map_err(|_| error("Plein écran indisponible."))?;
            return self.view();
        }
        if matches!(cmd, PlayerCommand::ConfirmTransfer) {
            let p = self
                .pending
                .as_ref()
                .ok_or_else(|| error("Aucun transfert proposé."))?;
            let sent = Instant::now();
            let status = p
                .connection
                .transport
                .confirm_transfer(&p.connection.profile_id, &p.offer.token)
                .await?;
            self.pending.as_mut().unwrap().confirmed = true;
            return self.finish_transfer(w, status, sent).await;
        }
        if let PlayerCommand::Next { expected_id } = &cmd {
            uuid_check(expected_id)?;
            let s = self
                .session
                .as_ref()
                .ok_or_else(|| error("Aucune proposition de suite."))?;
            if !s.snapshot.ended || s.snapshot.error.is_some() {
                return Err(error("La vidéo n'est pas terminée."));
            }
            let next = s
                .transport
                .next_video(&s.profile, &s.info.media.id, &s.context)
                .await?;
            let media = next
                .media
                .as_ref()
                .ok_or_else(|| error("Il n'y a plus de vidéo suivante."))?;
            if media.id != *expected_id {
                self.session.as_mut().unwrap().next = Some(next);
                return Err(error("L'ordre a changé. Vérifiez la nouvelle proposition."));
            }
            if !next.available || !media.available {
                return Err(error("Vidéo suivante indisponible."));
            }
            let c = PlaybackConnection {
                transport: s.transport.clone(),
                profile_id: s.profile.clone(),
                device_id: s.device.clone(),
                server_id: String::new(),
                cache: s.cache,
            };
            let context = s.context.clone();
            let rect = s.rect;
            let id = media.id.clone();
            return self.start(w, c, id, rect, Some(context)).await;
        }
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
                // Pausing is a local playback state, not the end of this
                // session.  Keep the lease and libmpv instance alive so its
                // bounded packet cache can continue to be useful on resume.
                // Reporting `playing: false` also makes the watch clock stop
                // accruing credit while the user is paused.
                if s.lease.is_some() {
                    let saved = report(s, was_playing, false).await;
                    saved?;
                }
            }
            PlayerCommand::Resume => {
                if s.lease.is_none() {
                    let sent = Instant::now();
                    let acquired = s
                        .transport
                        .acquire_playback(
                            &s.profile,
                            &AcquirePlayback {
                                media_id: s.info.media.id.clone(),
                                device_id: s.device.clone(),
                                context: s.context.clone(),
                            },
                        )
                        .await;
                    let g = match acquired {
                        Ok(g) => g,
                        Err(chill_client::ClientError::Api { code, .. })
                            if code == "PLAYBACK_ACTIVE" =>
                        {
                            let c = PlaybackConnection {
                                transport: s.transport.clone(),
                                profile_id: s.profile.clone(),
                                device_id: s.device.clone(),
                                server_id: String::new(),
                                cache: s.cache,
                            };
                            let media = s.info.media.id.clone();
                            let rect = s.rect;
                            let context = s.context.clone();
                            return self.start(w, c, media, rect, Some(context)).await;
                        }
                        Err(e) => return Err(e.into()),
                    };
                    s.clock = WatchClock::default();
                    let loaded = native_player::load(
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
                        s.cache,
                    )
                    .await;
                    s.lease = Some(g.lease);
                    s.controls =
                        Some(s.transport.subscribe_controls(
                            s.profile.clone(),
                            s.lease.as_ref().unwrap().clone(),
                        ));
                    s.snapshot = match loaded {
                        Ok(snapshot) => snapshot,
                        Err(e) => {
                            let _ = release(s).await;
                            return Err(native_error(e));
                        }
                    };
                    s.started = false;
                    s.last_beat = Instant::now();
                    s.desired_position = None;
                    s.next = None;
                    if let Err(e) =
                        native_player::action(w, NativeAction::Mount { rect: s.rect }).await
                    {
                        let _ = native_player::action(w, NativeAction::Close).await;
                        let _ = release(s).await;
                        return Err(native_error(e));
                    }
                } else {
                    renew(s, w).await?;
                    if s.snapshot.loaded {
                        s.snapshot =
                            native_player::action(w, NativeAction::Pause { paused: false })
                                .await
                                .map_err(native_error)?;
                    }
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
                } else if s.info.media.series.is_some() {
                    let g = s
                        .transport
                        .acquire_playback(
                            &s.profile,
                            &AcquirePlayback {
                                media_id: s.info.media.id.clone(),
                                device_id: s.device.clone(),
                                context: s.context.clone(),
                            },
                        )
                        .await?;
                    let saved = s
                        .transport
                        .save_playback_speed(&s.profile, &control(&g.lease), speed)
                        .await;
                    let released = s
                        .transport
                        .release_playback(
                            &s.profile,
                            &ReleasePlayback {
                                lease: control(&g.lease),
                                progress: None,
                            },
                        )
                        .await;
                    saved?;
                    released?;
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
                s.audio_override = Some(track_id);
            }
            PlayerCommand::Subtitle { track_id } => {
                s.selected_external = None;
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
                s.subtitle_override = Some(track_id);
            }
            PlayerCommand::Fullscreen { enabled } => {
                w.set_fullscreen(
                    enabled.unwrap_or(
                        !w.is_fullscreen()
                            .map_err(|_| error("Plein écran indisponible."))?,
                    ),
                )
                .map_err(|_| error("Plein écran indisponible."))?;
            }
            PlayerCommand::Volume { volume } => {
                if volume > 100 {
                    return Err(error("Volume invalide."));
                }
                s.snapshot = native_player::action(w, NativeAction::Volume { volume })
                    .await
                    .map_err(native_error)?;
            }
            PlayerCommand::ExternalSubtitle { id } => {
                s.subtitle_override = None;
                if !s.snapshot.loaded {
                    return Err(error("Attendez le chargement de la vidéo."));
                }
                if s.lease.is_some() {
                    add_external(s, w, &id).await?;
                } else {
                    let g = s
                        .transport
                        .acquire_playback(
                            &s.profile,
                            &AcquirePlayback {
                                media_id: s.info.media.id.clone(),
                                device_id: s.device.clone(),
                                context: s.context.clone(),
                            },
                        )
                        .await?;
                    s.lease = Some(g.lease);
                    let added = add_external(s, w, &id).await;
                    let released = release(s).await;
                    added?;
                    released?;
                }
            }
            _ => return Err(error("Commande de lecture invalide.")),
        }
        self.view()
    }
    pub async fn stop(&mut self, w: &WebviewWindow) -> Result<(), UiError> {
        self.stop_session(w, true).await
    }
    async fn stop_session(
        &mut self,
        w: &WebviewWindow,
        leave_fullscreen: bool,
    ) -> Result<(), UiError> {
        if leave_fullscreen {
            w.set_fullscreen(false)
                .map_err(|_| error("Plein écran indisponible."))?;
        }
        self.rendering.store(false, Ordering::Release);
        self.last_error = None;
        if let Some(p) = self.pending.take()
            && p.confirmed
            && let Ok(status) = p
                .connection
                .transport
                .transfer_status(&p.connection.profile_id, &p.offer.token)
                .await
            && let Some(lease) = status.lease
        {
            let _ = p
                .connection
                .transport
                .release_playback(
                    &p.connection.profile_id,
                    &ReleasePlayback {
                        lease: control(&lease),
                        progress: None,
                    },
                )
                .await;
        }
        if let Some(mut s) = self.session.take() {
            if let Some(task) = s.external_task.take() {
                task.abort();
            }
            if let Ok(snapshot) = native_player::action(w, NativeAction::Poll).await {
                s.snapshot = snapshot;
            }
            let playing = !s.snapshot.paused;
            let _ = native_player::action(w, NativeAction::Close).await;
            if s.lease.is_some() {
                let _ = report(&mut s, playing, false).await;
                let _ = release(&mut s).await;
            }
            for (_, path) in s.subtitle_cache {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
        Ok(())
    }
    async fn finish_transfer(
        &mut self,
        w: &WebviewWindow,
        status: TransferStatus,
        sent: Instant,
    ) -> Result<Value, UiError> {
        match status.phase {
            TransferPhase::AwaitingAck => {
                if let Some(p) = self.pending.as_mut() {
                    p.remaining_ms = status.remaining_ms;
                }
                self.view()
            }
            TransferPhase::Expired => {
                self.pending = None;
                self.last_error = Some("Le transfert a expiré. Relancez la vidéo.".into());
                self.view()
            }
            TransferPhase::Activated => {
                let p = self
                    .pending
                    .take()
                    .ok_or_else(|| error("Transfert annulé."))?;
                let lease = status
                    .lease
                    .ok_or_else(|| error("Réponse de transfert invalide."))?;
                if lease.profile_id != p.connection.profile_id
                    || lease.device_id != p.connection.device_id
                    || lease.media_id != p.info.media.id
                {
                    return Err(error("Réponse de transfert invalide."));
                }
                let personal = status
                    .personal
                    .ok_or_else(|| error("Reprise de transfert absente."))?;
                self.install(
                    w,
                    p.connection,
                    p.info,
                    p.rect,
                    p.context,
                    PlaybackGrant { lease, personal },
                    sent,
                )
                .await
            }
        }
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
    if uuid::Uuid::parse_str(s).is_err() {
        Err(error("Identifiant vidéo invalide."))
    } else {
        Ok(())
    }
}
fn language_equal(a: &str, b: &str) -> bool {
    let a = language_key(a);
    let b = language_key(b);
    a.is_some() && a == b
}
// Both integrated and external tracks use the same language comparison.
// Empty/invalid input is disabled; valid unknown codes remain distinct so an
// absent preference triggers the visible fallback, never a None==None match.
pub(crate) fn language_key(s: &str) -> Option<String> {
    let normalized = s.trim().split(['-', '_']).next()?.to_ascii_lowercase();
    if !(2..=3).contains(&normalized.len()) || !normalized.bytes().all(|b| b.is_ascii_alphabetic())
    {
        return None;
    }
    Some(
        match normalized.as_str() {
            "eng" => "en",
            "fra" | "fre" => "fr",
            "deu" | "ger" => "de",
            "spa" => "es",
            "ita" => "it",
            "jpn" => "ja",
            "por" => "pt",
            "zho" | "chi" => "zh",
            "rus" => "ru",
            "nld" | "dut" => "nl",
            "kor" => "ko",
            s => s,
        }
        .to_owned(),
    )
}
async fn add_external(s: &mut Session, w: &WebviewWindow, id: &str) -> Result<(), UiError> {
    let candidate = s
        .external
        .iter()
        .find(|t| t.id == id && !t.unavailable)
        .cloned()
        .ok_or_else(|| error("Sous-titre absent."))?;
    if let Some((_, path)) = s.subtitle_cache.iter().find(|(key, _)| key == id) {
        s.snapshot = native_player::action(
            w,
            NativeAction::AddSubtitle {
                path: path.clone(),
                title: candidate.title,
                language: candidate.language,
            },
        )
        .await
        .map_err(native_error)?;
        s.selected_external = Some(id.into());
        return Ok(());
    }
    if s.subtitle_cache.len() >= 4 {
        return Err(error(
            "Cache de sous-titres plein pour cette lecture. Fermez puis rouvrez la vidéo.",
        ));
    }
    let lease = s
        .lease
        .as_ref()
        .ok_or_else(|| error("Bail de lecture absent."))?;
    let bytes = s.transport.download_subtitle(&s.profile, lease, id).await?;
    apply_downloaded_external(s, w, candidate, bytes).await
}

fn matching_external<'a>(
    external: &'a [ExternalSubtitle],
    language: &str,
) -> Vec<&'a ExternalSubtitle> {
    external
        .iter()
        .filter(|t| {
            !t.unavailable
                && t.language
                    .as_deref()
                    .is_some_and(|l| language_equal(l, language))
        })
        .collect()
}

fn discover_external(s: &mut Session) {
    if let Some(task) = s.external_task.take() {
        task.abort();
    }
    let Some(lease) = s.lease.clone() else {
        return;
    };
    let transport = s.transport.clone();
    let profile = s.profile.clone();
    let language = s.info.preferences.subtitle_language.clone();
    let selected_before = s.selected_external.clone();
    let manual = s.subtitle_override.is_some();
    let integrated = s.snapshot.tracks.iter().any(|t| {
        t.kind == TrackKind::Subtitle
            && t.language
                .as_deref()
                .is_some_and(|l| language_equal(l, &language))
    });
    s.external_task = Some(tokio::spawn(async move {
        let external = transport.external_subtitles(&profile, &lease).await?;
        let matches = matching_external(&external, &language);
        let chosen = if manual {
            None
        } else {
            selected_before
                .as_ref()
                .and_then(|id| external.iter().find(|t| &t.id == id && !t.unavailable))
                .or_else(|| (!integrated && matches.len() == 1).then(|| matches[0]))
        };
        let preferred = if let Some(candidate) = chosen {
            let bytes = transport
                .download_subtitle(&profile, &lease, &candidate.id)
                .await
                .map_err(UiError::from);
            Some((candidate.clone(), bytes))
        } else {
            None
        };
        Ok(DiscoveredSubtitles {
            external,
            selected_before,
            preferred,
        })
    }));
}

async fn apply_downloaded_external(
    s: &mut Session,
    w: &WebviewWindow,
    candidate: ExternalSubtitle,
    bytes: Vec<u8>,
) -> Result<(), UiError> {
    if s.lease.is_none() {
        return Err(error("Bail de lecture absent."));
    }
    let id = candidate.id.clone();
    if let Some((_, path)) = s.subtitle_cache.iter().find(|(key, _)| key == &id) {
        s.snapshot = native_player::action(
            w,
            NativeAction::AddSubtitle {
                path: path.clone(),
                title: candidate.title,
                language: candidate.language,
            },
        )
        .await
        .map_err(native_error)?;
        s.selected_external = Some(id);
        return Ok(());
    }
    if s.subtitle_cache.len() >= 4 {
        return Err(error(
            "Cache de sous-titres plein pour cette lecture. Fermez puis rouvrez la vidéo.",
        ));
    }
    let dir = s.cache_dir.clone();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|_| error("Cache indisponible."))?;
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|_| error("Cache indisponible."))?;
    let mut total = 0u64;
    let mut count = 0;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|_| error("Cache indisponible."))?
    {
        count += 1;
        if count > 256 {
            return Err(error("Cache de sous-titres saturé."));
        }
        let meta = entry
            .metadata()
            .await
            .map_err(|_| error("Cache indisponible."))?;
        total = total.saturating_add(meta.len());
    }
    if total.saturating_add(bytes.len() as u64) > 64 * 1024 * 1024 {
        return Err(error("Cache de sous-titres saturé (64 MiB)."));
    }
    let path = dir.join(format!("{}.subtitle", uuid::Uuid::new_v4()));
    let mut opts = tokio::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    opts.mode(0o600);
    use tokio::io::AsyncWriteExt;
    let mut file = opts
        .open(&path)
        .await
        .map_err(|_| error("Cache indisponible."))?;
    if file.write_all(&bytes).await.is_err() {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(error("Cache indisponible."));
    }
    drop(file);
    s.subtitle_cache.push((id.clone(), path.clone()));
    s.snapshot = native_player::action(
        w,
        NativeAction::AddSubtitle {
            path,
            title: candidate.title,
            language: candidate.language,
        },
    )
    .await
    .map_err(native_error)?;
    s.selected_external = Some(id);
    Ok(())
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
    let obs = observation(s, playing, seek);
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
fn observation(s: &Session, playing: bool, seek: bool) -> Observation {
    Observation {
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
    }
}
async fn release(s: &mut Session) -> Result<(), UiError> {
    if let Some(task) = s.external_task.take() {
        task.abort();
    }
    s.controls = None;
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

/// A local failure cannot revoke a remote lease while the network is down, but
/// it must immediately stop every local renewal/control task and forget the
/// capability. The server then expires its lease at the previously fixed
/// deadline instead of the paused client continuing to hold it locally.
fn abandon(s: &mut Session) {
    if let Some(task) = s.external_task.take() {
        task.abort();
    }
    s.controls = None;
    s.lease = None;
    s.clock.forget_credit();
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
                    context: s.context.clone(),
                },
            )
            .await?;
        s.clock = WatchClock::default();
        s.lease = Some(g.lease);
        let saved = report(s, false, true).await;
        let released = release(s).await;
        saved?;
        released?;
    } else {
        s.snapshot = native_player::action(w, NativeAction::Seek { seconds: target })
            .await
            .map_err(native_error)?;
        report(s, !s.snapshot.paused, true).await?;
    }
    Ok(())
}

#[cfg(test)]
mod language_tests {
    use super::*;
    #[test]
    fn supported_and_absent_preferences_have_one_shared_key() {
        for (a, b) in [
            ("en-US", "eng"),
            (" FR_ca ", "fre"),
            ("de", "ger"),
            ("JA", "jpn"),
            ("pt-BR", "por"),
        ] {
            assert!(language_equal(a, b));
        }
        assert_eq!(language_key("ja").as_deref(), Some("ja"));
        assert_ne!(language_key("ja"), language_key("eng"));
        assert!(!language_equal("", ""));
        assert!(!language_equal("und", "ja"));
        assert!(!language_equal("unknown", "unknown"));
        assert_eq!(language_key(""), None);
        assert_eq!(language_key("../../"), None);
    }
}
