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
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSOpenGLContext, NSOpenGLPFAAccelerated, NSOpenGLPFAAlphaSize, NSOpenGLPFAColorSize,
    NSOpenGLPFAOpenGLProfile, NSOpenGLPixelFormat, NSOpenGLProfileVersion3_2Core, NSOpenGLView,
    NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ffi::{CStr, c_int, c_uint, c_void};
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};
use tauri::{Runtime, WebviewWindow};

const GL_FRAMEBUFFER: c_uint = 0x8D40;
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
    Mount { rect: NativeRect },
    Poll,
    LeaseDeadline { remaining_ms: u64 },
    Pause { paused: bool },
    Seek { seconds: f64 },
    Speed { speed: f64 },
    Audio { track_id: i64 },
    Subtitle { track_id: Option<i64> },
    Volume { volume: u8 },
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
    pub decoder: Option<String>,
    pub hardware_decoder: Option<String>,
    #[cfg(debug_assertions)]
    pub debug_nonblank: Option<bool>,
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
pub async fn load<R: Runtime>(
    window: &WebviewWindow<R>,
    transport: Transport,
    playback: PlaybackStream,
    resume_seconds: Option<f64>,
    audio_language: Option<String>,
    subtitle_language: Option<String>,
    speed: f64,
    lease: LeaseDeadline,
) -> Result<NativeSnapshot, NativePlayerError> {
    on_main(window, move |view| {
        let mut state = SurfacePlayer::new(view, transport, playback, lease)?;
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
        PLAYER.with(|slot| *slot.borrow_mut() = Some(state.clone()));
        state.borrow_mut().snapshot()
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

async fn on_main<R: Runtime, F>(
    window: &WebviewWindow<R>,
    op: F,
) -> Result<NativeSnapshot, NativePlayerError>
where
    F: FnOnce(*mut c_void) -> Result<NativeSnapshot, NativePlayerError> + Send + 'static,
{
    let (send, recv) = mpsc::sync_channel(1);
    let view = window
        .ns_view()
        .map_err(|e| NativePlayerError::Surface(e.to_string()))? as usize;
    window
        .run_on_main_thread(move || {
            let _ = send.send(op(view as *mut c_void));
        })
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    tauri::async_runtime::spawn_blocking(move || {
        recv.recv().map_err(|_| NativePlayerError::Disconnected)?
    })
    .await
    .map_err(|_| NativePlayerError::Disconnected)?
}

async fn on_main_unit<R: Runtime, F>(
    window: &WebviewWindow<R>,
    op: F,
) -> Result<(), NativePlayerError>
where
    F: FnOnce(*mut c_void) -> Result<(), NativePlayerError> + Send + 'static,
{
    let (send, recv) = mpsc::sync_channel(1);
    let view = window
        .ns_view()
        .map_err(|e| NativePlayerError::Surface(e.to_string()))? as usize;
    window
        .run_on_main_thread(move || {
            let _ = send.send(op(view as *mut c_void));
        })
        .map_err(|e| NativePlayerError::Surface(e.to_string()))?;
    tauri::async_runtime::spawn_blocking(move || {
        recv.recv().map_err(|_| NativePlayerError::Disconnected)?
    })
    .await
    .map_err(|_| NativePlayerError::Disconnected)?
}

struct SurfacePlayer {
    player: Player,
    renderer: OpenGlRenderer,
    view: Retained<NSOpenGLView>,
    context: Retained<NSOpenGLContext>,
    framebuffer: Framebuffer,
    media_id: String,
    paused: bool,
    ended: bool,
    error: Option<String>,
    loaded: bool,
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
    ) -> Result<Self, NativePlayerError> {
        let parent = unsafe { &*(parent as *mut NSView) };
        let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(1., 1.));
        let view = open_gl_view(rect)?;
        parent.addSubview(&view);
        view.setHidden(true);
        view.setWantsBestResolutionOpenGLSurface(true);
        let context = view
            .openGLContext()
            .ok_or_else(|| NativePlayerError::Surface("contexte OpenGL absent".into()))?;
        context.makeCurrentContext();
        let mut options = PlayerOptions::default();
        options.extra.insert("vo".into(), "libmpv".into());
        let mut player = Player::new(options)?;
        let factory = HttpMediaFactory::new(transport);
        factory.set_playback(playback.clone());
        player.register_stream("chill-media", std::sync::Arc::new(factory))?;
        let renderer = player.create_opengl_renderer(gl_proc)?;
        let now = SystemTime::now();
        Ok(Self {
            player,
            renderer,
            view,
            context,
            framebuffer: Framebuffer::new(1, 1)?,
            media_id: playback.media_id,
            paused: false,
            ended: false,
            error: None,
            loaded: false,
            pending: None,
            speed: 1.,
            volume: 100,
            deadline_instant: Instant::now() + lease.remaining,
            deadline_wall: now + lease.remaining,
        })
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
            NativeAction::Volume { volume } => {
                self.volume = volume.min(100);
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
        let parent = unsafe { self.view.superview() }
            .ok_or_else(|| NativePlayerError::Surface("surface détachée".into()))?;
        let bounds = parent.bounds();
        let scale = parent
            .window()
            .map(|w| w.backingScaleFactor())
            .unwrap_or(1.);
        let w = css.width.max(1.).min(bounds.size.width);
        let h = css.height.max(1.).min(bounds.size.height);
        let x = css.x.max(0.).min((bounds.size.width - w).max(0.));
        let y = (bounds.size.height - css.y - h)
            .max(0.)
            .min((bounds.size.height - h).max(0.));
        self.view
            .setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)));
        self.context.makeCurrentContext();
        self.framebuffer = Framebuffer::new((w * scale).ceil() as i32, (h * scale).ceil() as i32)?;
        self.view.setHidden(false);
        Ok(())
    }
    fn expired(&self) -> bool {
        Instant::now() >= self.deadline_instant || SystemTime::now() >= self.deadline_wall
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
        if !self.paused && !self.ended && !self.view.isHidden() {
            self.context.makeCurrentContext();
            self.framebuffer.clear();
            self.renderer
                .render(&mut self.framebuffer.descriptor(), true)?;
            self.context.flushBuffer();
        }
        Ok(())
    }
    fn poll_and_render(&mut self) -> Result<(), NativePlayerError> {
        self.expire_if_needed()?;
        match self.player.poll_event(Duration::ZERO) {
            PlayerEvent::FileLoaded => self.apply_pending()?,
            PlayerEvent::Ended { .. } | PlayerEvent::Shutdown => self.ended = true,
            _ => {}
        };
        self.render_tick()
    }
    fn apply_pending(&mut self) -> Result<(), NativePlayerError> {
        let Some(pending) = self.pending.take() else {
            self.loaded = true;
            return Ok(());
        };
        if let Some(position) = pending
            .resume_seconds
            .filter(|value| value.is_finite() && *value >= 0.0)
        {
            self.player.seek_absolute(position)?;
        }
        self.player.set_speed(pending.speed)?;
        self.speed = pending.speed;
        for track in self.player.tracks()? {
            if track.language.as_deref() == pending.audio_language.as_deref()
                && matches!(track.kind, chill_player::TrackKind::Audio)
            {
                self.player.select_audio(track.id)?;
            }
            if track.language.as_deref() == pending.subtitle_language.as_deref()
                && matches!(track.kind, chill_player::TrackKind::Subtitle)
            {
                self.player.select_subtitle(Some(track.id))?;
            }
        }
        self.loaded = true;
        Ok(())
    }
    fn snapshot(&mut self) -> Result<NativeSnapshot, NativePlayerError> {
        let _ = self.poll_and_render();
        Ok(NativeSnapshot {
            loaded: self.loaded,
            position: self.player.position_seconds()?,
            duration: self.player.duration_seconds()?,
            paused: self.paused,
            buffering: false,
            speed: self.speed,
            tracks: self.player.tracks()?,
            ended: self.ended,
            error: self.error.clone(),
            decoder: self.player.decoder()?,
            hardware_decoder: self.player.hardware_decoder()?,
            #[cfg(debug_assertions)]
            debug_nonblank: Some(self.framebuffer.nonblank()),
        })
    }
}

struct PendingLoad {
    resume_seconds: Option<f64>,
    audio_language: Option<String>,
    subtitle_language: Option<String>,
    speed: f64,
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
        self.id != 0
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
