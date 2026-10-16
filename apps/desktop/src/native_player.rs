//! AppKit-owned libmpv surface.  This deliberately has no `#[tauri::command]`:
//! the UI bridge may only pass a server-issued grant and a pinned `Transport`.
#![cfg(target_os = "macos")]
#![allow(deprecated)]

use chill_client::Transport;
use chill_player::{
    HttpMediaFactory, OpenGlFramebuffer, OpenGlRenderer, PlaybackStream, Player, PlayerError,
    PlayerEvent, PlayerOptions, Track,
};
use objc2::rc::Retained;
#[cfg(debug_assertions)]
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly};
use objc2::{msg_send, runtime::AnyClass};
use objc2_app_kit::{
    NSOpenGLContext, NSOpenGLPFAAccelerated, NSOpenGLPFAAlphaSize, NSOpenGLPFAColorSize,
    NSOpenGLPFADoubleBuffer, NSOpenGLPFAOpenGLProfile, NSOpenGLPixelFormat,
    NSOpenGLProfileVersion3_2Core, NSOpenGLView, NSView,
};
use objc2_foundation::{NSNumber, NSPoint, NSRect, NSSize, NSString};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ffi::{CStr, c_int, c_uint, c_void};
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};
use tauri::{Runtime, WebviewWindow};

const GL_FRAMEBUFFER: c_uint = 0x8D40;
const GL_READ_FRAMEBUFFER: c_uint = 0x8CA8;
const GL_DRAW_FRAMEBUFFER: c_uint = 0x8CA9;
const GL_TEXTURE_2D: c_uint = 0x0DE1;
const GL_COLOR_ATTACHMENT0: c_uint = 0x8CE0;
const GL_RGBA8: c_int = 0x8058;
const GL_RGBA: c_uint = 0x1908;
const GL_UNSIGNED_BYTE: c_uint = 0x1401;
const GL_LINEAR: c_int = 0x2601;
const GL_TEXTURE_MIN_FILTER: c_uint = 0x2801;
const GL_TEXTURE_MAG_FILTER: c_uint = 0x2800;
const GL_FRAMEBUFFER_COMPLETE: c_uint = 0x8CD5;

#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {
    fn glGenFramebuffers(n: c_int, ids: *mut c_uint);
    fn glBindFramebuffer(t: c_uint, id: c_uint);
    fn glDeleteFramebuffers(n: c_int, ids: *const c_uint);
    fn glGenTextures(n: c_int, ids: *mut c_uint);
    fn glBindTexture(t: c_uint, id: c_uint);
    fn glDeleteTextures(n: c_int, ids: *const c_uint);
    fn glTexParameteri(t: c_uint, p: c_uint, v: c_int);
    fn glTexImage2D(
        t: c_uint,
        l: c_int,
        i: c_int,
        w: c_int,
        h: c_int,
        b: c_int,
        f: c_uint,
        k: c_uint,
        p: *const c_void,
    );
    fn glFramebufferTexture2D(t: c_uint, a: c_uint, tt: c_uint, tex: c_uint, l: c_int);
    fn glCheckFramebufferStatus(t: c_uint) -> c_uint;
    fn glViewport(x: c_int, y: c_int, w: c_int, h: c_int);
    fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    fn glClear(mask: c_uint);
    fn glBlitFramebuffer(
        sx0: c_int,
        sy0: c_int,
        sx1: c_int,
        sy1: c_int,
        dx0: c_int,
        dy0: c_int,
        dx1: c_int,
        dy1: c_int,
        mask: c_uint,
        filter: c_uint,
    );
    fn glReadPixels(
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        format: c_uint,
        kind: c_uint,
        pixels: *mut c_void,
    );
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NativeRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NativeAction {
    Mount {
        rect: NativeRect,
    },
    Poll,
    LeaseDeadline {
        remaining_ms: u64,
    },
    Pause {
        paused: bool,
    },
    Seek {
        seconds: f64,
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
    AddSubtitle {
        path: std::path::PathBuf,
        title: String,
        language: Option<String>,
    },
    Volume {
        volume: u8,
    },
    Fullscreen,
    Close,
}
#[derive(Debug, Clone, Copy)]
pub struct LeaseDeadline {
    pub remaining: Duration,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSnapshot {
    pub loaded: bool,
    pub position: Option<f64>,
    pub duration: Option<f64>,
    pub paused: bool,
    pub buffering: bool,
    pub speed: f64,
    pub tracks: Vec<Track>,
    pub ended: bool,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub decoder: Option<String>,
    pub hardware_decoder: Option<String>,
    pub fullscreen: bool,
    pub volume: u8,
    #[cfg(debug_assertions)]
    pub debug_nonblank: Option<bool>,
    #[cfg(debug_assertions)]
    pub debug_surface: NativeSurfaceGeometry,
}
#[cfg(debug_assertions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSurfaceGeometry {
    pub frame: NativeRect,
    pub bounds: NativeRect,
    pub screen: Option<NativeRect>,
    pub screen_safe_bounds: Option<NativeRect>,
    pub webview_above_video: bool,
    pub webview_receives_pointer: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum NativePlayerError {
    #[error("lecteur natif: {0}")]
    Player(#[from] PlayerError),
    #[error("surface native: {0}")]
    Surface(String),
    #[error("action native interrompue")]
    Disconnected,
}

thread_local! { static PLAYER: RefCell<Option<Rc<RefCell<SurfacePlayer>>>> = const { RefCell::new(None) }; }

/// Loads only the already-authorised stream; callers cannot provide a URL or filesystem path.
// Grant, transport and preferences are explicit at the AppKit ownership boundary.
#[allow(clippy::too_many_arguments)]
pub async fn load<R: Runtime>(
    window: &WebviewWindow<R>,
    transport: Transport,
    playback: PlaybackStream,
    resume_seconds: Option<f64>,
    audio_language: Option<String>,
    subtitle_language: Option<String>,
    speed: f64,
    lease: LeaseDeadline,
    cache: chill_client::playback_cache::PlaybackCache,
) -> Result<NativeSnapshot, NativePlayerError> {
    on_main(window, move |view| {
        PLAYER.with(|slot| {
            let existing = slot.borrow().clone();
            if let Some(state) = existing {
                let mut state = state.borrow_mut();
                state.reload(
                    transport,
                    playback,
                    resume_seconds,
                    audio_language,
                    subtitle_language,
                    speed,
                    lease,
                    cache,
                )?;
                state.snapshot()
            } else {
                let mut state = SurfacePlayer::new(view, transport, playback, lease, cache)?;
                state
                    .player
                    .load(&format!("chill-media://{}", state.media_id))?;
                state.pending = Some(PendingLoad {
                    resume_seconds,
                    audio_language,
                    subtitle_language,
                    speed,
                });
                let state = Rc::new(RefCell::new(state));
                *slot.borrow_mut() = Some(state.clone());
                state.borrow_mut().snapshot()
            }
        })
    })
    .await
}

pub async fn action<R: Runtime>(
    window: &WebviewWindow<R>,
    action: NativeAction,
) -> Result<NativeSnapshot, NativePlayerError> {
    on_main(window, move |_view| {
        PLAYER.with(|slot| {
            let state = slot
                .borrow()
                .clone()
                .ok_or_else(|| NativePlayerError::Surface("aucun lecteur chargé".into()))?;
            let mut state = state.borrow_mut();
            state.apply(action)
        })
    })
    .await
}

/// Hot-path frame pulse. It obtains no tracks/properties and serializes no snapshot.
/// The Tauri coordinator schedules it only while the surface is visible and active.
pub async fn render_tick<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), NativePlayerError> {
    on_main_unit(window, |_view| {
        PLAYER.with(|slot| {
            if let Some(state) = slot.borrow().clone() {
                state.borrow_mut().render_tick()?;
            }
            Ok(())
        })
    })
    .await
}

/// Safety pulse independent of rendering, UI dispatch and the host's network mutex.
pub async fn deadline_tick<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), NativePlayerError> {
    on_main_unit(window, |_view| {
        PLAYER.with(|slot| {
            if let Some(state) = slot.borrow().clone() {
                state.borrow_mut().expire_if_needed()?;
            }
            Ok(())
        })
    })
    .await
}
pub async fn close_if_loaded<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<(), NativePlayerError> {
    on_main_unit(window, |_view| {
        PLAYER.with(|slot| {
            if let Some(state) = slot.borrow().clone() {
                let mut state = state.borrow_mut();
                state.player.pause(true)?;
                state.paused = true;
                state.view.setHidden(true);
            }
            Ok(())
        })
    })
    .await
}
#[cfg(debug_assertions)]
pub async fn guard_paused<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<bool, NativePlayerError> {
    // Read the native guard flag only. A normal snapshot/poll expires the player
    // itself and would mask a broken independent watchdog in the regression.
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .run_on_main_thread(move || {
            let out = PLAYER.with(|slot| slot.borrow().as_ref().map(|state| state.borrow().paused));
            let _ = tx.send(out);
        })
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    rx.await
        .map_err(|_| NativePlayerError::Disconnected)?
        .ok_or_else(|| NativePlayerError::Surface("aucun lecteur chargé".into()))
}
#[cfg(debug_assertions)]
pub fn qa_prepare_visible_player<R: Runtime>(
    window: &tauri::Webview<R>,
) -> Result<(), NativePlayerError> {
    window
        .run_on_main_thread(|| {
            if let Some(app_class) = AnyClass::get(c"NSApplication") {
                let app: *mut AnyObject = unsafe { msg_send![app_class, sharedApplication] };
                if !app.is_null() {
                    unsafe {
                        let _: () = msg_send![app, unhide: std::ptr::null::<AnyObject>()];
                        let _: () = msg_send![app, activateIgnoringOtherApps: true];
                    }
                }
            }
        })
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    window
        .show()
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    window
        .set_focus()
        .map_err(|e| NativePlayerError::Surface(e.to_string()))
}
fn deadline_has_passed(
    now: Instant,
    wall: SystemTime,
    deadline: Instant,
    wall_deadline: SystemTime,
) -> bool {
    now >= deadline || wall >= wall_deadline
}

async fn on_main<R: Runtime, F>(
    window: &WebviewWindow<R>,
    op: F,
) -> Result<NativeSnapshot, NativePlayerError>
where
    F: FnOnce(*mut c_void) -> Result<NativeSnapshot, NativePlayerError> + Send + 'static,
{
    let (send, recv) = tokio::sync::oneshot::channel();
    let view = window
        .ns_view()
        .map_err(|e| NativePlayerError::Surface(e.to_string()))? as usize;
    window
        .run_on_main_thread(move || {
            let _ = send.send(op(view as *mut c_void));
        })
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    recv.await.map_err(|_| NativePlayerError::Disconnected)?
}

async fn on_main_unit<R: Runtime, F>(
    window: &WebviewWindow<R>,
    op: F,
) -> Result<(), NativePlayerError>
where
    F: FnOnce(*mut c_void) -> Result<(), NativePlayerError> + Send + 'static,
{
    let (send, recv) = tokio::sync::oneshot::channel();
    let view = window
        .ns_view()
        .map_err(|e| NativePlayerError::Surface(e.to_string()))? as usize;
    window
        .run_on_main_thread(move || {
            let _ = send.send(op(view as *mut c_void));
        })
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    recv.await.map_err(|_| NativePlayerError::Disconnected)?
}

struct SurfacePlayer {
    player: Player,
    factory: HttpMediaFactory,
    renderer: OpenGlRenderer,
    view: Retained<NSOpenGLView>,
    overlay: Retained<NSView>,
    context: Retained<NSOpenGLContext>,
    framebuffer: Framebuffer,
    drawable_size: (i32, i32),
    media_id: String,
    paused: bool,
    ended: bool,
    error: Option<String>,
    notice: Option<String>,
    loaded: bool,
    load_phase: LoadPhase,
    pending: Option<PendingLoad>,
    speed: f64,
    volume: u8,
    deadline_instant: Instant,
    deadline_wall: SystemTime,
}
impl SurfacePlayer {
    fn new(
        parent: *mut c_void,
        transport: Transport,
        playback: PlaybackStream,
        lease: LeaseDeadline,
        cache: chill_client::playback_cache::PlaybackCache,
    ) -> Result<Self, NativePlayerError> {
        let parent = unsafe { &*(parent as *mut NSView) };
        let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(1., 1.));
        let view = open_gl_view(rect)?;
        // Wry's WKWebView and the renderer are siblings in the content NSView.
        // Keep the WKWebView above OpenGL: transparent DOM exposes the image,
        // while real web controls and all pointer/key events stay in WebKit.
        let webview_class = AnyClass::get(c"WKWebView")
            .ok_or_else(|| NativePlayerError::Surface("webview native absente".into()))?;
        let overlay = parent
            .subviews()
            .iter()
            .find(|child| unsafe { msg_send![&**child, isKindOfClass: webview_class] })
            .ok_or_else(|| NativePlayerError::Surface("webview native absente".into()))?;
        unsafe {
            let no = NSNumber::numberWithBool(false);
            let key = NSString::from_str("drawsBackground");
            let _: () = msg_send![&*overlay, setValue: &*no, forKey: &*key];
            let _: () =
                msg_send![parent, addSubview: &*view, positioned: -1isize, relativeTo: &*overlay];
        }
        view.setHidden(true);
        view.setWantsBestResolutionOpenGLSurface(true);
        let context = view
            .openGLContext()
            .ok_or_else(|| NativePlayerError::Surface("contexte OpenGL absent".into()))?;
        context.makeCurrentContext();
        let mut options = PlayerOptions {
            cache,
            ..PlayerOptions::default()
        };
        options.extra.insert("vo".into(), "libmpv".into());
        let mut player = Player::new(options)?;
        // Never decode/play before the server-confirmed heartbeat allows the bridge to unpause.
        player.pause(true)?;
        let factory = HttpMediaFactory::new(transport);
        factory.set_playback(playback.clone());
        player.register_stream("chill-media", std::sync::Arc::new(factory.clone()))?;
        let renderer = player.create_opengl_renderer(gl_proc)?;
        let now = SystemTime::now();
        Ok(Self {
            player,
            factory,
            renderer,
            view,
            overlay,
            context,
            framebuffer: Framebuffer::new(1, 1)?,
            drawable_size: (1, 1),
            media_id: playback.media_id,
            paused: true,
            ended: false,
            error: None,
            notice: None,
            loaded: false,
            load_phase: LoadPhase::Loading,
            pending: None,
            speed: 1.,
            volume: 100,
            deadline_instant: Instant::now() + lease.remaining,
            deadline_wall: now + lease.remaining,
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn reload(
        &mut self,
        transport: Transport,
        playback: PlaybackStream,
        resume_seconds: Option<f64>,
        audio_language: Option<String>,
        subtitle_language: Option<String>,
        speed: f64,
        lease: LeaseDeadline,
        cache: chill_client::playback_cache::PlaybackCache,
    ) -> Result<(), NativePlayerError> {
        // The registered callback observes this factory; replacing the active grant does not
        // destroy libmpv or its existing HTTP worker on the AppKit main thread.
        self.player.pause(true)?;
        self.paused = true;
        self.player.set_cache_limits(cache)?;
        self.factory.clear_playback();
        self.factory.set_transport(transport);
        self.factory.set_playback(playback.clone());
        self.media_id = playback.media_id;
        self.ended = false;
        self.error = None;
        self.notice = None;
        self.loaded = false;
        self.load_phase = LoadPhase::Loading;
        // libmpv presents asynchronously.  Do not leave the previous movie in
        // the AppKit drawable while the next grant is being acquired/decoded.
        // This is a real OpenGL clear + flush, not a CSS mask that could hide
        // the new native frame as well.
        self.present_black();
        self.pending = Some(PendingLoad {
            resume_seconds,
            audio_language,
            subtitle_language,
            speed,
        });
        self.deadline_instant = Instant::now() + lease.remaining;
        self.deadline_wall = SystemTime::now() + lease.remaining;
        self.player
            .load(&format!("chill-media://{}", self.media_id))?;
        Ok(())
    }
    fn apply(&mut self, action: NativeAction) -> Result<NativeSnapshot, NativePlayerError> {
        self.expire_if_needed()?;
        match action {
            NativeAction::Mount { rect } => self.mount(rect)?,
            NativeAction::Poll => self.poll_and_render()?,
            NativeAction::LeaseDeadline { remaining_ms } => {
                let d = Duration::from_millis(remaining_ms);
                self.deadline_instant = Instant::now() + d;
                self.deadline_wall = SystemTime::now() + d
            }
            NativeAction::Pause { paused } => {
                if !paused && self.expired() {
                    return Err(NativePlayerError::Surface("bail expiré".into()));
                }
                self.player.pause(paused)?;
                self.paused = paused
            }
            NativeAction::Seek { seconds } => self.player.seek_absolute(seconds.max(0.))?,
            NativeAction::Speed { speed } => {
                self.player.set_speed(speed)?;
                self.speed = speed
            }
            NativeAction::Audio { track_id } => self.player.select_audio(track_id)?,
            NativeAction::Subtitle { track_id } => self.player.select_subtitle(track_id)?,
            NativeAction::AddSubtitle {
                path,
                title,
                language,
            } => {
                let path = path.to_str().ok_or_else(|| {
                    NativePlayerError::Surface("cache sous-titre invalide".into())
                })?;
                self.player
                    .add_subtitle(path, Some(&title), language.as_deref())?;
            }
            NativeAction::Volume { volume } => {
                self.volume = volume.min(100);
                self.player.set_volume(self.volume)?;
            }
            NativeAction::Fullscreen => {}
            NativeAction::Close => {
                self.view.setHidden(true);
                self.paused = true;
                self.player.pause(true)?;
            }
        };
        self.snapshot()
    }
    fn mount(&mut self, css: NativeRect) -> Result<(), NativePlayerError> {
        if ![css.x, css.y, css.width, css.height]
            .into_iter()
            .all(f64::is_finite)
            || css.width <= 0.
            || css.height <= 0.
            || css.width > 16384.
            || css.height > 16384.
        {
            return Err(NativePlayerError::Surface(
                "rectangle lecteur invalide".into(),
            ));
        }
        let parent = unsafe { self.view.superview() }
            .ok_or_else(|| NativePlayerError::Surface("surface détachée".into()))?;
        let bounds = parent.bounds();
        let scale = parent
            .window()
            .map(|w| w.backingScaleFactor())
            .unwrap_or(1.);
        let w = css.width.min(bounds.size.width).max(1.);
        let h = css.height.min(bounds.size.height).max(1.);
        let x = css.x.max(0.).min((bounds.size.width - w).max(0.));
        let y = (bounds.size.height - css.y - h)
            .max(0.)
            .min((bounds.size.height - h).max(0.));
        self.view
            .setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)));
        // Mounts also restore ordering after a native full-screen transition.
        unsafe {
            let _: () = msg_send![&*parent, addSubview: &*self.view, positioned: -1isize, relativeTo: &*self.overlay];
        }
        self.context.makeCurrentContext();
        self.drawable_size = ((w * scale).ceil() as i32, (h * scale).ceil() as i32);
        let (fw, fh) = framebuffer_size(self.drawable_size.0, self.drawable_size.1);
        if self.framebuffer.w != fw || self.framebuffer.h != fh {
            self.framebuffer = Framebuffer::new(fw, fh)?;
        }
        self.view.setHidden(false);
        if self.loaded {
            self.render_frame()?;
        } else {
            self.present_black();
        }
        Ok(())
    }
    fn expired(&self) -> bool {
        deadline_has_passed(
            Instant::now(),
            SystemTime::now(),
            self.deadline_instant,
            self.deadline_wall,
        )
    }
    fn expire_if_needed(&mut self) -> Result<(), NativePlayerError> {
        if self.expired() && !self.paused {
            self.player.pause(true)?;
            self.paused = true;
            self.error = Some("bail expiré".into())
        }
        Ok(())
    }
    fn render_tick(&mut self) -> Result<(), NativePlayerError> {
        self.expire_if_needed()?;
        if should_render_frame(self.loaded, self.paused, self.ended, self.view.isHidden()) {
            self.render_frame()?;
        }
        Ok(())
    }
    fn render_frame(&mut self) -> Result<(), NativePlayerError> {
        self.context.makeCurrentContext();
        self.framebuffer.clear();
        self.renderer
            .render(&mut self.framebuffer.descriptor(), true)?;
        unsafe {
            glBindFramebuffer(GL_READ_FRAMEBUFFER, self.framebuffer.id);
            glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0);
            glBlitFramebuffer(
                0,
                0,
                self.framebuffer.w,
                self.framebuffer.h,
                0,
                0,
                self.drawable_size.0,
                self.drawable_size.1,
                0x4000,
                GL_LINEAR as c_uint,
            );
        }
        self.context.flushBuffer();
        Ok(())
    }
    fn present_black(&self) {
        self.context.makeCurrentContext();
        unsafe {
            glBindFramebuffer(GL_DRAW_FRAMEBUFFER, 0);
            glViewport(0, 0, self.drawable_size.0, self.drawable_size.1);
            glClearColor(0., 0., 0., 1.);
            glClear(0x4000);
        }
        self.context.flushBuffer();
    }
    fn poll_and_render(&mut self) -> Result<(), NativePlayerError> {
        self.expire_if_needed()?;
        for _ in 0..32 {
            let event = self.player.poll_event(Duration::ZERO);
            match self.load_phase.observe(&event) {
                EventDisposition::Ignore => continue,
                EventDisposition::Started => continue,
                EventDisposition::FileLoaded => self.apply_pending()?,
                EventDisposition::Process => match event {
                    PlayerEvent::None => break,
                    PlayerEvent::Ended { reason, error } => {
                        self.ended = true;
                        if reason != 0 || error != 0 {
                            self.error = Some(format!("lecture interrompue ({reason}/{error})"));
                        }
                    }
                    PlayerEvent::Shutdown => {
                        self.ended = true;
                        self.error = Some("moteur arrêté".into());
                    }
                    _ => {}
                },
            }
        }
        self.render_tick()
    }
    fn apply_pending(&mut self) -> Result<(), NativePlayerError> {
        let Some(pending) = self.pending.take() else {
            self.loaded = true;
            return Ok(());
        };
        if let Some(position) =
            resume_position(pending.resume_seconds, self.player.duration_seconds()?)
        {
            self.player.seek_absolute(position)?;
        }
        self.player.set_speed(pending.speed)?;
        self.speed = pending.speed;
        let tracks = self.player.tracks()?;
        let wanted_subtitle = pending.subtitle_language.as_deref().and_then(language_key);
        let (audio, audio_unavailable) =
            preferred_audio(&tracks, pending.audio_language.as_deref());
        if let Some(track) = audio {
            self.player.select_audio(track.id)?;
        }
        if audio_unavailable {
            self.notice = Some(
                "Langue audio préférée indisponible : audio original ou première piste utilisé."
                    .into(),
            );
        }
        if let Some(wanted) = wanted_subtitle {
            if let Some(track) = tracks.iter().find(|track| {
                matches!(track.kind, chill_player::TrackKind::Subtitle)
                    && track.language.as_deref().and_then(language_key).as_deref()
                        == Some(wanted.as_str())
            }) {
                self.player.select_subtitle(Some(track.id))?;
            } else {
                self.player.select_subtitle(None)?;
                self.notice =
                    Some("Sous-titres préférés indisponibles : sous-titres désactivés.".into());
            }
        } else {
            self.player.select_subtitle(None)?;
            if pending
                .subtitle_language
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty())
            {
                self.notice =
                    Some("Langue de sous-titres non reconnue : sous-titres désactivés.".into());
            }
        }
        self.loaded = true;
        self.load_phase = LoadPhase::Steady;
        Ok(())
    }
    fn snapshot(&mut self) -> Result<NativeSnapshot, NativePlayerError> {
        let _ = self.poll_and_render();
        Ok(NativeSnapshot {
            loaded: self.loaded,
            position: self.player.position_seconds()?,
            duration: self.player.duration_seconds()?,
            paused: self.paused,
            buffering: self.player.is_buffering()?,
            speed: self.speed,
            tracks: self.player.tracks()?,
            ended: self.ended,
            error: self.error.clone(),
            notice: self.notice.clone(),
            decoder: self.player.decoder()?,
            hardware_decoder: self.player.hardware_decoder()?,
            fullscreen: self.view.window().is_some_and(|w| {
                let mask: usize = unsafe { msg_send![&*w, styleMask] };
                mask & (1 << 14) != 0 // NSWindowStyleMaskFullScreen
            }),
            volume: self.volume,
            #[cfg(debug_assertions)]
            debug_nonblank: Some(self.framebuffer.nonblank()),
            #[cfg(debug_assertions)]
            debug_surface: self.surface_geometry(),
        })
    }
    #[cfg(debug_assertions)]
    fn surface_geometry(&self) -> NativeSurfaceGeometry {
        let rect = |r: NSRect| NativeRect {
            x: r.origin.x,
            y: r.origin.y,
            width: r.size.width,
            height: r.size.height,
        };
        let parent = unsafe { self.view.superview() }.expect("mounted surface");
        let views = parent.subviews();
        let index = |view: &NSView| views.iter().position(|v| std::ptr::eq(&*v, view));
        let frame = self.view.frame();
        let point = NSPoint::new(
            frame.origin.x + frame.size.width / 2.,
            frame.origin.y + frame.size.height / 2.,
        );
        let hit = parent.hitTest(point);
        NativeSurfaceGeometry {
            frame: rect(frame),
            bounds: rect(parent.bounds()),
            screen: parent
                .window()
                .and_then(|window| window.screen())
                .map(|screen| rect(screen.frame())),
            // AppKit reserves the camera housing/menu strip in native full
            // screen. The QA oracle uses this usable area, not physical pixels
            // that the window intentionally leaves outside its content view.
            screen_safe_bounds: parent
                .window()
                .and_then(|window| window.screen())
                .map(|screen| {
                    let frame = screen.frame();
                    let inset = screen.safeAreaInsets();
                    rect(NSRect::new(
                        NSPoint::new(frame.origin.x + inset.left, frame.origin.y + inset.bottom),
                        NSSize::new(
                            frame.size.width - inset.left - inset.right,
                            frame.size.height - inset.top - inset.bottom,
                        ),
                    ))
                }),
            webview_above_video: index(&self.view)
                .zip(index(&self.overlay))
                .is_some_and(|(video, web)| video < web),
            webview_receives_pointer: hit.is_some_and(|hit| hit.isDescendantOf(&self.overlay)),
        }
    }
}

fn framebuffer_size(width: i32, height: i32) -> (i32, i32) {
    let factor = (8192. / f64::from(width))
        .min(8192. / f64::from(height))
        .min(1.);
    (
        (f64::from(width) * factor).round() as i32,
        (f64::from(height) * factor).round() as i32,
    )
}

/// A new load cannot repaint until libmpv has emitted `FileLoaded` and the
/// generation's pending preferences have been applied.  This keeps a cleared
/// drawable black instead of resurrecting an old frame during the hand-off.
fn should_render_frame(loaded: bool, paused: bool, ended: bool, hidden: bool) -> bool {
    loaded && !paused && !ended && !hidden
}

#[cfg(test)]
mod deadline_clock_tests {
    use super::*;
    #[test]
    fn oversized_framebuffers_keep_ratio_and_normal_retina_size() {
        assert_eq!(framebuffer_size(3840, 2160), (3840, 2160));
        assert_eq!(framebuffer_size(15360, 8640), (8192, 4608));
        assert_eq!(framebuffer_size(8640, 15360), (4608, 8192));
    }

    #[test]
    fn framebuffer_stays_black_until_the_new_generation_is_loaded() {
        assert!(!should_render_frame(false, false, false, false));
        assert!(!should_render_frame(true, true, false, false));
        assert!(!should_render_frame(true, false, true, false));
        assert!(!should_render_frame(true, false, false, true));
        assert!(should_render_frame(true, false, false, false));
    }
    #[test]
    fn wall_clock_expiry_covers_a_monotonic_clock_that_did_not_advance_during_suspend() {
        let now = Instant::now();
        let wall = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        assert!(deadline_has_passed(
            now,
            wall,
            now + Duration::from_secs(30),
            wall - Duration::from_secs(1)
        ));
    }
    #[test]
    fn backward_wall_clock_never_extends_monotonic_expiry() {
        let now = Instant::now();
        let wall = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        assert!(deadline_has_passed(
            now,
            wall,
            now - Duration::from_secs(1),
            wall + Duration::from_secs(30)
        ));
        assert!(!deadline_has_passed(
            now,
            wall,
            now + Duration::from_secs(1),
            wall + Duration::from_secs(1)
        ));
    }
}

fn resume_position(position: Option<f64>, duration: Option<f64>) -> Option<f64> {
    position
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|position| {
            // FileLoaded supplies the actual duration. An EOF saved by another
            // device is a replay, not a seek into an already-ended native file.
            if duration.is_some_and(|d| {
                d.is_finite() && d > 0.0 && position > 0.0 && position >= (d - 0.25).max(0.0)
            }) {
                0.0
            } else {
                position
            }
        })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadPhase {
    Loading,
    WaitingForFile,
    Steady,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventDisposition {
    Ignore,
    Started,
    FileLoaded,
    Process,
}
impl LoadPhase {
    /// An Ended before this load's Started belongs to the superseded file.
    fn observe(&mut self, event: &PlayerEvent) -> EventDisposition {
        match (*self, event) {
            (Self::Loading, PlayerEvent::Started) => {
                *self = Self::WaitingForFile;
                EventDisposition::Started
            }
            (Self::Loading, PlayerEvent::Ended { .. } | PlayerEvent::FileLoaded) => {
                EventDisposition::Ignore
            }
            (Self::WaitingForFile, PlayerEvent::FileLoaded) => EventDisposition::FileLoaded,
            (Self::WaitingForFile, PlayerEvent::Ended { .. }) => EventDisposition::Process,
            (_, _) => EventDisposition::Process,
        }
    }
}

#[cfg(test)]
mod reload_phase_tests {
    use super::*;
    #[test]
    fn a_completed_position_restarts_but_partial_playback_keeps_its_resume() {
        assert_eq!(resume_position(Some(16.0), Some(16.0)), Some(0.0));
        assert_eq!(resume_position(Some(16.0), Some(16.034)), Some(0.0));
        assert_eq!(resume_position(Some(99.0), Some(100.0)), Some(99.0));
        assert_eq!(resume_position(Some(18.5), None), Some(18.5));
        assert_eq!(resume_position(Some(f64::NAN), Some(100.0)), None);
    }
    #[test]
    fn old_stop_before_new_started_is_ignored() {
        let mut phase = LoadPhase::Loading;
        assert_eq!(
            phase.observe(&PlayerEvent::Ended {
                reason: 2,
                error: 0
            }),
            EventDisposition::Ignore
        );
        assert_eq!(
            phase.observe(&PlayerEvent::Started),
            EventDisposition::Started
        );
        assert_eq!(
            phase.observe(&PlayerEvent::FileLoaded),
            EventDisposition::FileLoaded
        );
    }
    #[test]
    fn new_load_failure_is_not_masked() {
        let mut phase = LoadPhase::Loading;
        assert_eq!(
            phase.observe(&PlayerEvent::Started),
            EventDisposition::Started
        );
        assert_eq!(
            phase.observe(&PlayerEvent::Ended {
                reason: 2,
                error: -13
            }),
            EventDisposition::Process
        );
    }
    #[test]
    fn normal_eof_after_loaded_is_processed() {
        let mut phase = LoadPhase::Steady;
        assert_eq!(
            phase.observe(&PlayerEvent::Ended {
                reason: 0,
                error: 0
            }),
            EventDisposition::Process
        );
    }
    #[test]
    fn repeated_reloads_each_ignore_only_pre_start_stop() {
        for _ in 0..3 {
            let mut phase = LoadPhase::Loading;
            assert_eq!(
                phase.observe(&PlayerEvent::Ended {
                    reason: 2,
                    error: 0
                }),
                EventDisposition::Ignore
            );
            assert_eq!(
                phase.observe(&PlayerEvent::Started),
                EventDisposition::Started
            );
            assert_eq!(
                phase.observe(&PlayerEvent::FileLoaded),
                EventDisposition::FileLoaded
            );
            phase = LoadPhase::Steady;
            assert_eq!(
                phase.observe(&PlayerEvent::Ended {
                    reason: 0,
                    error: 0
                }),
                EventDisposition::Process
            );
        }
    }
}

struct PendingLoad {
    resume_seconds: Option<f64>,
    audio_language: Option<String>,
    subtitle_language: Option<String>,
    speed: f64,
}
fn language_key(language: &str) -> Option<String> {
    crate::player_bridge::language_key(language)
}
fn preferred_audio<'a>(tracks: &'a [Track], requested: Option<&str>) -> (Option<&'a Track>, bool) {
    let wanted = requested.and_then(language_key);
    let chosen = wanted
        .as_ref()
        .and_then(|wanted| {
            tracks.iter().find(|t| {
                t.kind == chill_player::TrackKind::Audio
                    && t.language.as_deref().and_then(language_key).as_ref() == Some(wanted)
            })
        })
        .or_else(|| {
            tracks
                .iter()
                .find(|t| t.kind == chill_player::TrackKind::Audio && t.original)
        })
        .or_else(|| {
            tracks
                .iter()
                .find(|t| t.kind == chill_player::TrackKind::Audio)
        });
    let unavailable = requested.is_some_and(|s| !s.trim().is_empty())
        && (wanted.is_none()
            || chosen.is_none_or(|t| t.language.as_deref().and_then(language_key) != wanted));
    (chosen, unavailable)
}
fn open_gl_view(rect: NSRect) -> Result<Retained<NSOpenGLView>, NativePlayerError> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| NativePlayerError::Surface("AppKit hors thread principal".into()))?;
    let mut a = [
        NSOpenGLPFAAccelerated,
        NSOpenGLPFAOpenGLProfile,
        NSOpenGLProfileVersion3_2Core,
        NSOpenGLPFAColorSize,
        24,
        NSOpenGLPFAAlphaSize,
        8,
        NSOpenGLPFADoubleBuffer,
        0,
    ];
    let p = NonNull::new(a.as_mut_ptr())
        .ok_or_else(|| NativePlayerError::Surface("attributs OpenGL absents".into()))?;
    let f = unsafe { NSOpenGLPixelFormat::initWithAttributes(NSOpenGLPixelFormat::alloc(), p) }
        .ok_or_else(|| NativePlayerError::Surface("format OpenGL refusé".into()))?;
    NSOpenGLView::initWithFrame_pixelFormat(NSOpenGLView::alloc(mtm), rect, Some(&f))
        .ok_or_else(|| NativePlayerError::Surface("NSOpenGLView refusée".into()))
}
fn gl_proc(name: &CStr) -> *mut c_void {
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
}
struct Framebuffer {
    id: c_uint,
    tex: c_uint,
    w: c_int,
    h: c_int,
}
impl Framebuffer {
    fn new(w: c_int, h: c_int) -> Result<Self, NativePlayerError> {
        let (mut id, mut tex) = (0, 0);
        unsafe {
            glGenFramebuffers(1, &mut id);
            glBindFramebuffer(GL_FRAMEBUFFER, id);
            glGenTextures(1, &mut tex);
            glBindTexture(GL_TEXTURE_2D, tex);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                w,
                h,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                ptr::null(),
            );
            glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, tex, 0);
            if glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE {
                return Err(NativePlayerError::Surface(
                    "framebuffer OpenGL incomplet".into(),
                ));
            }
        };
        Ok(Self { id, tex, w, h })
    }
    fn descriptor(&self) -> OpenGlFramebuffer {
        OpenGlFramebuffer {
            fbo: self.id as c_int,
            width: self.w,
            height: self.h,
            internal_format: GL_RGBA8,
        }
    }
    fn clear(&self) {
        unsafe {
            glBindFramebuffer(GL_FRAMEBUFFER, self.id);
            glViewport(0, 0, self.w, self.h);
            glClearColor(0., 0., 0., 1.);
            glClear(0x4000)
        }
    }
    fn nonblank(&self) -> bool {
        // Debug-only assertion signal: sample actual FBO pixels, never resource identity.
        let mut samples = [0u8; 4 * 9];
        unsafe {
            glBindFramebuffer(GL_READ_FRAMEBUFFER, self.id);
            for (index, (x, y)) in [
                (0, 0),
                (self.w / 2, 0),
                (self.w - 1, 0),
                (0, self.h / 2),
                (self.w / 2, self.h / 2),
                (self.w - 1, self.h / 2),
                (0, self.h - 1),
                (self.w / 2, self.h - 1),
                (self.w - 1, self.h - 1),
            ]
            .into_iter()
            .enumerate()
            {
                glReadPixels(
                    x.max(0),
                    y.max(0),
                    1,
                    1,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    samples[index * 4..].as_mut_ptr().cast(),
                );
            }
        }
        samples
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
    }
}
impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            glDeleteTextures(1, &self.tex);
            glDeleteFramebuffers(1, &self.id)
        }
    }
}

#[cfg(test)]
mod preferred_audio_tests {
    use super::*;
    use chill_player::TrackKind;
    #[test]
    fn non_normalizable_preference_never_matches_an_untagged_track_before_original() {
        let audio = |id, language: Option<&str>, original| Track {
            id,
            kind: TrackKind::Audio,
            language: language.map(str::to_owned),
            title: None,
            selected: false,
            external: false,
            original,
        };
        let tracks = vec![
            audio(1, None, false),
            audio(2, Some("eng"), true),
            audio(3, Some("fra"), false),
        ];
        let (chosen, notice) = preferred_audio(&tracks, Some("Japanese"));
        assert_eq!(chosen.unwrap().id, 2);
        assert!(notice);
        let (chosen, notice) = preferred_audio(&tracks, Some("ja"));
        assert_eq!(chosen.unwrap().id, 2);
        assert!(notice);
        let (chosen, notice) = preferred_audio(&tracks, Some("fr-CA"));
        assert_eq!(chosen.unwrap().id, 3);
        assert!(!notice);
        let (chosen, notice) = preferred_audio(&tracks, Some(""));
        assert_eq!(chosen.unwrap().id, 2);
        assert!(!notice);
        let (chosen, notice) = preferred_audio(&tracks[0..1], Some("Japanese"));
        assert_eq!(chosen.unwrap().id, 1);
        assert!(notice);
        let (chosen, notice) = preferred_audio(&[], Some("en"));
        assert!(chosen.is_none());
        assert!(notice);
    }
}
