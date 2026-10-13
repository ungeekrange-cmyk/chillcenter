//! Dynamically loaded libmpv adapter.
//!
//! macOS does not support mpv's `wid` embedding. The AppKit layer must create
//! and make current an OpenGL context for its `NSView`, then call
//! [`Player::create_opengl_renderer`] and [`OpenGlRenderer::render`].

use chill_client::{ClientError, Transport};
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use thiserror::Error;

const MPV_ERROR_LOADING_FAILED: i32 = -13;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("libmpv introuvable dans les emplacements contrôlés")]
    LibraryNotFound,
    #[error("symbole libmpv absent: {0}")]
    MissingSymbol(String),
    #[error("API client libmpv {major}.{minor} trop ancienne (minimum 1.106)")]
    UnsupportedApi { major: u64, minor: u64 },
    #[error("libmpv a refusé l'opération ({0})")]
    Mpv(i32),
    #[error("valeur contenant un octet nul")]
    InvalidCString,
    #[error("option libmpv non autorisée: {0}")]
    UnsafeOption(String),
    #[error("schéma de flux invalide: {0}")]
    InvalidScheme(String),
    #[error("aucune lecture active configurée")]
    NoActivePlayback,
    #[error("erreur de flux: {0}")]
    Stream(String),
    #[error("l'API de rendu OpenGL est absente de cette libmpv")]
    RenderApiUnavailable,
}

impl From<ClientError> for PlayerError {
    fn from(value: ClientError) -> Self {
        Self::Stream(value.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeWindowHandle {
    AppKitNsView(usize),
    Win32Hwnd(usize),
    X11Window(u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerOptions {
    pub native_window: Option<NativeWindowHandle>,
    pub hardware_decoding: bool,
    pub initial_volume: u8,
    pub extra: BTreeMap<String, String>,
}

impl Default for PlayerOptions {
    fn default() -> Self {
        Self {
            native_window: None,
            hardware_decoding: true,
            initial_volume: 100,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub kind: TrackKind,
    pub language: Option<String>,
    pub title: Option<String>,
    pub selected: bool,
    pub external: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Audio,
    Subtitle,
    Video,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerEvent {
    None,
    Shutdown,
    Started,
    FileLoaded,
    Ended { reason: i32, error: i32 },
    Seek,
    PlaybackRestarted,
    VideoReconfigured,
    AudioReconfigured,
    PropertyChanged,
    QueueOverflow,
    Unknown(i32),
}

#[derive(Clone, Default)]
pub struct CancellationToken {
    state: Arc<(AtomicBool, Mutex<()>, Condvar)>,
}

impl CancellationToken {
    pub fn cancel(&self) {
        self.state.0.store(true, Ordering::Release);
        self.state.2.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        self.state.0.load(Ordering::Acquire)
    }
    pub fn wait_cancelled(&self, timeout: Duration) -> bool {
        if self.is_cancelled() {
            return true;
        }
        let guard = self.state.1.lock().expect("cancellation lock poisoned");
        let _ = self
            .state
            .2
            .wait_timeout_while(guard, timeout, |_| !self.is_cancelled());
        self.is_cancelled()
    }
}

pub trait MediaSource: Send {
    fn read_at(
        &mut self,
        offset: u64,
        buffer: &mut [u8],
        cancellation: &CancellationToken,
    ) -> Result<usize, PlayerError>;
    fn size(&self) -> Option<u64>;
}

pub trait MediaFactory: Send + Sync {
    fn open(&self, uri: &str) -> Result<Box<dyn MediaSource>, PlayerError>;
}

#[derive(Clone, Debug)]
pub struct PlaybackStream {
    pub media_id: String,
    pub session_id: String,
    pub generation: u64,
}

#[derive(Clone)]
pub struct HttpMediaFactory {
    transport: Transport,
    active: Arc<RwLock<Option<PlaybackStream>>>,
}

impl HttpMediaFactory {
    pub fn new(transport: Transport) -> Self {
        Self {
            transport,
            active: Arc::new(RwLock::new(None)),
        }
    }
    pub fn set_playback(&self, playback: PlaybackStream) {
        *self.active.write().expect("playback lock poisoned") = Some(playback);
    }
    pub fn clear_playback(&self) {
        *self.active.write().expect("playback lock poisoned") = None;
    }
}

impl MediaFactory for HttpMediaFactory {
    fn open(&self, uri: &str) -> Result<Box<dyn MediaSource>, PlayerError> {
        let media_id = uri
            .strip_prefix("chill-media://")
            .unwrap_or(uri)
            .trim_start_matches('/');
        let active = self
            .active
            .read()
            .expect("playback lock poisoned")
            .clone()
            .ok_or(PlayerError::NoActivePlayback)?;
        if media_id != active.media_id {
            return Err(PlayerError::Stream(
                "le média ne correspond pas au bail actif".into(),
            ));
        }
        Ok(Box::new(HttpMediaSource {
            transport: self.transport.clone(),
            playback: active,
            etag: None,
            size: None,
        }))
    }
}

struct HttpMediaSource {
    transport: Transport,
    playback: PlaybackStream,
    etag: Option<String>,
    size: Option<u64>,
}

impl MediaSource for HttpMediaSource {
    fn read_at(
        &mut self,
        offset: u64,
        buffer: &mut [u8],
        cancellation: &CancellationToken,
    ) -> Result<usize, PlayerError> {
        if cancellation.is_cancelled() {
            return Err(PlayerError::Stream("lecture annulée".into()));
        }
        // A reqwest blocking body cannot be interrupted externally. Keep it
        // on a bounded worker so libmpv's cancel callback wakes this callback;
        // the worker owns its state and exits no later than Transport's timeout.
        let transport = self.transport.clone();
        let playback = self.playback.clone();
        let etag = self.etag.clone();
        let length = buffer.len();
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut bytes = vec![0; length];
            let result = transport.read_media_range_blocking(
                &playback.media_id,
                &playback.session_id,
                playback.generation,
                offset,
                &mut bytes,
                etag.as_deref(),
            );
            let _ = sender.send((result, bytes));
        });
        let (read, bytes) = loop {
            if cancellation.is_cancelled() {
                return Err(PlayerError::Stream("lecture annulée".into()));
            }
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(result) => break result,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(PlayerError::Stream("worker HTTP interrompu".into()));
                }
            }
        };
        let read = read?;
        buffer[..read.bytes_read].copy_from_slice(&bytes[..read.bytes_read]);
        if self.etag.is_none() {
            self.etag = read.etag;
        }
        if read.total_size.is_some() {
            self.size = read.total_size;
        }
        Ok(read.bytes_read)
    }
    fn size(&self) -> Option<u64> {
        self.size
    }
}

type MpvHandle = c_void;
type MpvRenderContext = c_void;
type MpvCreate = unsafe extern "C" fn() -> *mut MpvHandle;
type MpvInitialize = unsafe extern "C" fn(*mut MpvHandle) -> c_int;
type MpvTerminateDestroy = unsafe extern "C" fn(*mut MpvHandle);
type MpvClientApiVersion = unsafe extern "C" fn() -> u64;
type MpvSetOptionString =
    unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int;
type MpvSetPropertyString =
    unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int;
type MpvGetPropertyString = unsafe extern "C" fn(*mut MpvHandle, *const c_char) -> *mut c_char;
type MpvCommand = unsafe extern "C" fn(*mut MpvHandle, *const *const c_char) -> c_int;
type MpvWaitEvent = unsafe extern "C" fn(*mut MpvHandle, c_double) -> *const MpvEvent;
type MpvFree = unsafe extern "C" fn(*mut c_void);
type MpvStreamCbAddRo =
    unsafe extern "C" fn(*mut MpvHandle, *const c_char, *mut c_void, OpenFn) -> c_int;
type MpvRenderContextCreate =
    unsafe extern "C" fn(*mut *mut MpvRenderContext, *mut MpvHandle, *mut MpvRenderParam) -> c_int;
type MpvRenderContextRender =
    unsafe extern "C" fn(*mut MpvRenderContext, *mut MpvRenderParam) -> c_int;
type MpvRenderContextFree = unsafe extern "C" fn(*mut MpvRenderContext);

#[repr(C)]
struct MpvEvent {
    event_id: c_int,
    error: c_int,
    _reply_userdata: u64,
    data: *mut c_void,
}
#[repr(C)]
struct MpvEventEndFile {
    reason: c_int,
    error: c_int,
    _playlist_entry_id: i64,
    _playlist_insert_id: i64,
    _playlist_insert_num_entries: c_int,
}
#[repr(C)]
struct MpvRenderParam {
    kind: c_int,
    data: *mut c_void,
}
#[repr(C)]
struct MpvOpenGlInitParams {
    get_proc_address: GetProcAddress,
    get_proc_address_ctx: *mut c_void,
}
#[repr(C)]
pub struct OpenGlFramebuffer {
    pub fbo: c_int,
    pub width: c_int,
    pub height: c_int,
    pub internal_format: c_int,
}
#[repr(C)]
struct MpvStreamCbInfo {
    cookie: *mut c_void,
    read_fn: ReadFn,
    seek_fn: SeekFn,
    size_fn: SizeFn,
    close_fn: CloseFn,
    cancel_fn: CancelFn,
}
type OpenFn = unsafe extern "C" fn(*mut c_void, *mut c_char, *mut MpvStreamCbInfo) -> c_int;
type ReadFn = unsafe extern "C" fn(*mut c_void, *mut c_char, u64) -> i64;
type SeekFn = unsafe extern "C" fn(*mut c_void, i64) -> i64;
type SizeFn = unsafe extern "C" fn(*mut c_void) -> i64;
type CloseFn = unsafe extern "C" fn(*mut c_void);
type CancelFn = unsafe extern "C" fn(*mut c_void);
type GetProcAddress = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;

#[derive(Clone, Copy)]
struct Api {
    initialize: MpvInitialize,
    terminate_destroy: MpvTerminateDestroy,
    set_option_string: MpvSetOptionString,
    set_property_string: MpvSetPropertyString,
    get_property_string: MpvGetPropertyString,
    command: MpvCommand,
    wait_event: MpvWaitEvent,
    free: MpvFree,
    stream_cb_add_ro: MpvStreamCbAddRo,
    render_context_create: Option<MpvRenderContextCreate>,
    render_context_render: Option<MpvRenderContextRender>,
    render_context_free: Option<MpvRenderContextFree>,
}

struct Registry {
    factory: Arc<dyn MediaFactory>,
}
struct StreamCookie {
    source: Mutex<Box<dyn MediaSource>>,
    position: Mutex<u64>,
    cancellation: CancellationToken,
}

pub struct Player {
    handle: NonNull<MpvHandle>,
    api: Api,
    _library: Library,
    registries: Vec<*const Registry>,
    native_window: Option<NativeWindowHandle>,
}

impl Player {
    pub fn new(options: PlayerOptions) -> Result<Self, PlayerError> {
        let (library, _) = load_library()?;
        unsafe {
            let create: MpvCreate = symbol(&library, b"mpv_create\0")?;
            let api_version: MpvClientApiVersion = symbol(&library, b"mpv_client_api_version\0")?;
            let version = api_version();
            let (major, minor) = (version >> 16, version & 0xffff);
            if major < 1 || (major == 1 && minor < 106) {
                return Err(PlayerError::UnsupportedApi { major, minor });
            }
            let api = Api {
                initialize: symbol(&library, b"mpv_initialize\0")?,
                terminate_destroy: symbol(&library, b"mpv_terminate_destroy\0")?,
                set_option_string: symbol(&library, b"mpv_set_option_string\0")?,
                set_property_string: symbol(&library, b"mpv_set_property_string\0")?,
                get_property_string: symbol(&library, b"mpv_get_property_string\0")?,
                command: symbol(&library, b"mpv_command\0")?,
                wait_event: symbol(&library, b"mpv_wait_event\0")?,
                free: symbol(&library, b"mpv_free\0")?,
                stream_cb_add_ro: symbol(&library, b"mpv_stream_cb_add_ro\0")?,
                render_context_create: optional_symbol(&library, b"mpv_render_context_create\0"),
                render_context_render: optional_symbol(&library, b"mpv_render_context_render\0"),
                render_context_free: optional_symbol(&library, b"mpv_render_context_free\0"),
            };
            let handle = NonNull::new(create()).ok_or(PlayerError::Mpv(-2))?;
            let player = Self {
                handle,
                api,
                _library: library,
                registries: Vec::new(),
                native_window: options.native_window,
            };
            player.set_option("terminal", "no")?;
            player.set_option("input-default-bindings", "no")?;
            player.set_option("input-vo-keyboard", "no")?;
            player.set_option(
                "hwdec",
                if options.hardware_decoding {
                    "auto-safe"
                } else {
                    "no"
                },
            )?;
            player.set_option("volume", &options.initial_volume.min(100).to_string())?;
            #[cfg(not(target_os = "macos"))]
            if let Some(handle) = options.native_window {
                let value = match handle {
                    NativeWindowHandle::AppKitNsView(value)
                    | NativeWindowHandle::Win32Hwnd(value) => value as u64,
                    NativeWindowHandle::X11Window(value) => value,
                };
                player.set_option("wid", &value.to_string())?;
            }
            for (name, value) in options.extra {
                if !allowed_extra_option(&name) {
                    return Err(PlayerError::UnsafeOption(name));
                }
                player.set_option(&name, &value)?;
            }
            check((api.initialize)(handle.as_ptr()))?;
            Ok(player)
        }
    }

    pub fn native_window(&self) -> Option<NativeWindowHandle> {
        self.native_window
    }

    pub fn register_stream(
        &mut self,
        scheme: &str,
        factory: Arc<dyn MediaFactory>,
    ) -> Result<(), PlayerError> {
        if scheme.is_empty()
            || !scheme.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.')
            })
        {
            return Err(PlayerError::InvalidScheme(scheme.into()));
        }
        let scheme = CString::new(scheme).map_err(|_| PlayerError::InvalidCString)?;
        let raw = Arc::into_raw(Arc::new(Registry { factory }));
        let result = unsafe {
            (self.api.stream_cb_add_ro)(
                self.handle.as_ptr(),
                scheme.as_ptr(),
                raw.cast_mut().cast(),
                open_callback,
            )
        };
        if result < 0 {
            unsafe {
                drop(Arc::from_raw(raw));
            }
            return Err(PlayerError::Mpv(result));
        }
        self.registries.push(raw);
        Ok(())
    }

    pub fn load(&self, uri: &str) -> Result<(), PlayerError> {
        self.command(&["loadfile", uri, "replace"])
    }
    pub fn pause(&self, paused: bool) -> Result<(), PlayerError> {
        self.set_property("pause", if paused { "yes" } else { "no" })
    }
    pub fn seek_relative(&self, seconds: f64) -> Result<(), PlayerError> {
        self.command(&["seek", &seconds.to_string(), "relative+exact"])
    }
    pub fn seek_absolute(&self, seconds: f64) -> Result<(), PlayerError> {
        self.command(&["seek", &seconds.to_string(), "absolute+exact"])
    }
    pub fn set_speed(&self, speed: f64) -> Result<(), PlayerError> {
        if ![0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0].contains(&speed) {
            return Err(PlayerError::Mpv(-4));
        }
        self.set_property("speed", &speed.to_string())
    }
    pub fn select_audio(&self, id: i64) -> Result<(), PlayerError> {
        self.set_property("aid", &id.to_string())
    }
    pub fn select_subtitle(&self, id: Option<i64>) -> Result<(), PlayerError> {
        self.set_property("sid", &id.map_or_else(|| "no".into(), |id| id.to_string()))
    }
    pub fn add_subtitle(
        &self,
        uri: &str,
        title: Option<&str>,
        language: Option<&str>,
    ) -> Result<(), PlayerError> {
        self.command(&[
            "sub-add",
            uri,
            "select",
            title.unwrap_or(""),
            language.unwrap_or(""),
        ])
    }
    pub fn position_seconds(&self) -> Result<Option<f64>, PlayerError> {
        Ok(self.property("time-pos")?.and_then(|v| v.parse().ok()))
    }
    pub fn duration_seconds(&self) -> Result<Option<f64>, PlayerError> {
        Ok(self.property("duration")?.and_then(|v| v.parse().ok()))
    }
    pub fn decoder(&self) -> Result<Option<String>, PlayerError> {
        self.property("video-codec")
    }
    pub fn hardware_decoder(&self) -> Result<Option<String>, PlayerError> {
        self.property("hwdec-current")
    }

    pub fn tracks(&self) -> Result<Vec<Track>, PlayerError> {
        let count = self
            .property("track-list/count")?
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let mut tracks = Vec::with_capacity(count);
        for index in 0..count {
            let prefix = format!("track-list/{index}");
            let Some(id) = self
                .property(&format!("{prefix}/id"))?
                .and_then(|v| v.parse().ok())
            else {
                continue;
            };
            let kind = match self.property(&format!("{prefix}/type"))?.as_deref() {
                Some("audio") => TrackKind::Audio,
                Some("sub") => TrackKind::Subtitle,
                Some("video") => TrackKind::Video,
                _ => TrackKind::Unknown,
            };
            tracks.push(Track {
                id,
                kind,
                language: self.property(&format!("{prefix}/lang"))?,
                title: self.property(&format!("{prefix}/title"))?,
                selected: property_bool(self.property(&format!("{prefix}/selected"))?),
                external: property_bool(self.property(&format!("{prefix}/external"))?),
            });
        }
        Ok(tracks)
    }

    pub fn poll_event(&self, timeout: Duration) -> PlayerEvent {
        let pointer = unsafe { (self.api.wait_event)(self.handle.as_ptr(), timeout.as_secs_f64()) };
        if pointer.is_null() {
            return PlayerEvent::None;
        }
        let event = unsafe { &*pointer };
        match event.event_id {
            0 => PlayerEvent::None,
            1 => PlayerEvent::Shutdown,
            6 => PlayerEvent::Started,
            7 if !event.data.is_null() => {
                let end = unsafe { &*(event.data as *const MpvEventEndFile) };
                PlayerEvent::Ended {
                    reason: end.reason,
                    error: end.error,
                }
            }
            7 => PlayerEvent::Ended {
                reason: 0,
                error: event.error,
            },
            8 => PlayerEvent::FileLoaded,
            17 => PlayerEvent::VideoReconfigured,
            18 => PlayerEvent::AudioReconfigured,
            20 => PlayerEvent::Seek,
            21 => PlayerEvent::PlaybackRestarted,
            22 => PlayerEvent::PropertyChanged,
            24 => PlayerEvent::QueueOverflow,
            id => PlayerEvent::Unknown(id),
        }
    }

    pub fn create_opengl_renderer<F>(
        &self,
        get_proc_address: F,
    ) -> Result<OpenGlRenderer, PlayerError>
    where
        F: Fn(&CStr) -> *mut c_void + Send + Sync + 'static,
    {
        let create = self
            .api
            .render_context_create
            .ok_or(PlayerError::RenderApiUnavailable)?;
        let render = self
            .api
            .render_context_render
            .ok_or(PlayerError::RenderApiUnavailable)?;
        let free = self
            .api
            .render_context_free
            .ok_or(PlayerError::RenderApiUnavailable)?;
        let resolver_raw = Arc::into_raw(Arc::new(GlResolver {
            callback: Box::new(get_proc_address),
        }));
        let mut init = MpvOpenGlInitParams {
            get_proc_address: gl_get_proc_address,
            get_proc_address_ctx: resolver_raw.cast_mut().cast(),
        };
        let mut params = [
            MpvRenderParam {
                kind: 1,
                data: c"opengl".as_ptr().cast_mut().cast(),
            },
            MpvRenderParam {
                kind: 2,
                data: (&mut init as *mut MpvOpenGlInitParams).cast(),
            },
            MpvRenderParam {
                kind: 0,
                data: ptr::null_mut(),
            },
        ];
        let mut context = ptr::null_mut();
        let result = unsafe { create(&mut context, self.handle.as_ptr(), params.as_mut_ptr()) };
        if result < 0 || context.is_null() {
            unsafe {
                drop(Arc::from_raw(resolver_raw));
            }
            return Err(PlayerError::Mpv(result));
        }
        Ok(OpenGlRenderer {
            context: NonNull::new(context).expect("checked context"),
            render,
            free,
            resolver: resolver_raw,
        })
    }

    fn set_option(&self, name: &str, value: &str) -> Result<(), PlayerError> {
        unsafe { call_string_pair(self.handle, self.api.set_option_string, name, value) }
    }
    fn set_property(&self, name: &str, value: &str) -> Result<(), PlayerError> {
        unsafe { call_string_pair(self.handle, self.api.set_property_string, name, value) }
    }
    fn property(&self, name: &str) -> Result<Option<String>, PlayerError> {
        let name = CString::new(name).map_err(|_| PlayerError::InvalidCString)?;
        let raw = unsafe { (self.api.get_property_string)(self.handle.as_ptr(), name.as_ptr()) };
        if raw.is_null() {
            return Ok(None);
        }
        let value = unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() };
        unsafe { (self.api.free)(raw.cast()) };
        Ok(Some(value))
    }
    fn command(&self, parts: &[&str]) -> Result<(), PlayerError> {
        let strings = parts
            .iter()
            .map(|part| CString::new(*part).map_err(|_| PlayerError::InvalidCString))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = strings.iter().map(|part| part.as_ptr()).collect::<Vec<_>>();
        pointers.push(ptr::null());
        check(unsafe { (self.api.command)(self.handle.as_ptr(), pointers.as_ptr()) })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        unsafe {
            (self.api.terminate_destroy)(self.handle.as_ptr());
        }
        for registry in self.registries.drain(..) {
            unsafe {
                drop(Arc::from_raw(registry));
            }
        }
    }
}

struct GlResolver {
    callback: Box<dyn Fn(&CStr) -> *mut c_void + Send + Sync>,
}
pub struct OpenGlRenderer {
    context: NonNull<MpvRenderContext>,
    render: MpvRenderContextRender,
    free: MpvRenderContextFree,
    resolver: *const GlResolver,
}

impl OpenGlRenderer {
    /// The caller must make the AppKit view's OpenGL context current first.
    pub fn render(
        &mut self,
        framebuffer: &mut OpenGlFramebuffer,
        flip_y: bool,
    ) -> Result<(), PlayerError> {
        let mut flip = i32::from(flip_y);
        let mut params = [
            MpvRenderParam {
                kind: 3,
                data: (framebuffer as *mut OpenGlFramebuffer).cast(),
            },
            MpvRenderParam {
                kind: 4,
                data: (&mut flip as *mut i32).cast(),
            },
            MpvRenderParam {
                kind: 0,
                data: ptr::null_mut(),
            },
        ];
        check(unsafe { (self.render)(self.context.as_ptr(), params.as_mut_ptr()) })
    }
}
impl Drop for OpenGlRenderer {
    fn drop(&mut self) {
        unsafe {
            (self.free)(self.context.as_ptr());
            drop(Arc::from_raw(self.resolver));
        }
    }
}

unsafe extern "C" fn gl_get_proc_address(context: *mut c_void, name: *const c_char) -> *mut c_void {
    if context.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let resolver = unsafe { &*(context as *const GlResolver) };
    (resolver.callback)(unsafe { CStr::from_ptr(name) })
}

unsafe extern "C" fn open_callback(
    user_data: *mut c_void,
    uri: *mut c_char,
    info: *mut MpvStreamCbInfo,
) -> c_int {
    if user_data.is_null() || uri.is_null() || info.is_null() {
        return MPV_ERROR_LOADING_FAILED;
    }
    let raw = user_data as *const Registry;
    unsafe {
        Arc::increment_strong_count(raw);
    }
    let registry = unsafe { Arc::from_raw(raw) };
    let uri = unsafe { CStr::from_ptr(uri) }.to_string_lossy();
    match registry.factory.open(&uri) {
        Ok(source) => {
            let cookie = Arc::new(StreamCookie {
                source: Mutex::new(source),
                position: Mutex::new(0),
                cancellation: CancellationToken::default(),
            });
            unsafe {
                *info = MpvStreamCbInfo {
                    cookie: Arc::into_raw(cookie).cast_mut().cast(),
                    read_fn: read_callback,
                    seek_fn: seek_callback,
                    size_fn: size_callback,
                    close_fn: close_callback,
                    cancel_fn: cancel_callback,
                };
            }
            0
        }
        Err(_) => MPV_ERROR_LOADING_FAILED,
    }
}

unsafe fn clone_cookie(cookie: *mut c_void) -> Option<Arc<StreamCookie>> {
    if cookie.is_null() {
        return None;
    }
    let raw = cookie as *const StreamCookie;
    unsafe {
        Arc::increment_strong_count(raw);
    }
    Some(unsafe { Arc::from_raw(raw) })
}

unsafe extern "C" fn read_callback(cookie: *mut c_void, buffer: *mut c_char, count: u64) -> i64 {
    let Some(cookie) = (unsafe { clone_cookie(cookie) }) else {
        return -1;
    };
    if buffer.is_null() || count > isize::MAX as u64 {
        return -1;
    }
    let mut source = match cookie.source.lock() {
        Ok(source) => source,
        Err(_) => return -1,
    };
    let mut position = match cookie.position.lock() {
        Ok(position) => position,
        Err(_) => return -1,
    };
    let output = unsafe { std::slice::from_raw_parts_mut(buffer.cast::<u8>(), count as usize) };
    match source.read_at(*position, output, &cookie.cancellation) {
        Ok(read) => {
            *position = position.saturating_add(read as u64);
            read as i64
        }
        Err(_) => -1,
    }
}
unsafe extern "C" fn seek_callback(cookie: *mut c_void, offset: i64) -> i64 {
    let Some(cookie) = (unsafe { clone_cookie(cookie) }) else {
        return -1;
    };
    if offset < 0 {
        return -1;
    }
    match cookie.position.lock() {
        Ok(mut position) => {
            *position = offset as u64;
            offset
        }
        Err(_) => -1,
    }
}
unsafe extern "C" fn size_callback(cookie: *mut c_void) -> i64 {
    let Some(cookie) = (unsafe { clone_cookie(cookie) }) else {
        return -1;
    };
    match cookie.source.lock() {
        Ok(source) => source
            .size()
            .and_then(|size| i64::try_from(size).ok())
            .unwrap_or(-1),
        Err(_) => -1,
    }
}
unsafe extern "C" fn cancel_callback(cookie: *mut c_void) {
    if let Some(cookie) = unsafe { clone_cookie(cookie) } {
        cookie.cancellation.cancel();
    }
}
unsafe extern "C" fn close_callback(cookie: *mut c_void) {
    if !cookie.is_null() {
        unsafe {
            drop(Arc::from_raw(cookie as *const StreamCookie));
        }
    }
}

unsafe fn call_string_pair(
    handle: NonNull<MpvHandle>,
    function: unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int,
    name: &str,
    value: &str,
) -> Result<(), PlayerError> {
    let name = CString::new(name).map_err(|_| PlayerError::InvalidCString)?;
    let value = CString::new(value).map_err(|_| PlayerError::InvalidCString)?;
    check(unsafe { function(handle.as_ptr(), name.as_ptr(), value.as_ptr()) })
}
fn check(result: i32) -> Result<(), PlayerError> {
    if result < 0 {
        Err(PlayerError::Mpv(result))
    } else {
        Ok(())
    }
}
fn property_bool(value: Option<String>) -> bool {
    matches!(value.as_deref(), Some("yes" | "true" | "1"))
}
fn allowed_extra_option(name: &str) -> bool {
    matches!(
        name,
        "audio-language"
            | "sub-language"
            | "cache"
            | "cache-secs"
            | "video-sync"
            | "interpolation"
            | "vo"
            | "ao"
    )
}

unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, PlayerError> {
    unsafe { library.get::<T>(name) }
        .map(|symbol| *symbol)
        .map_err(|_| {
            PlayerError::MissingSymbol(
                String::from_utf8_lossy(&name[..name.len() - 1]).into_owned(),
            )
        })
}
fn optional_symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
    unsafe { library.get::<T>(name).ok().map(|symbol| *symbol) }
}

fn load_library() -> Result<(Library, PathBuf), PlayerError> {
    for path in controlled_library_paths() {
        if !path.exists() {
            continue;
        }
        if let Ok(library) = unsafe { Library::new(&path) } {
            return Ok((library, path));
        }
    }
    Err(PlayerError::LibraryNotFound)
}

fn controlled_library_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(contents) = executable.parent().and_then(Path::parent)
    {
        paths.push(contents.join("Frameworks/libmpv.dylib"));
    }
    paths.extend([
        PathBuf::from("/opt/homebrew/opt/mpv/lib/libmpv.dylib"),
        PathBuf::from("/opt/homebrew/lib/libmpv.dylib"),
        PathBuf::from("/usr/local/lib/libmpv.dylib"),
        PathBuf::from("/usr/lib/libmpv.so.2"),
        PathBuf::from("/usr/lib/aarch64-linux-gnu/libmpv.so.2"),
    ]);
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::thread;
    use std::time::Instant;

    struct BlockingSource;
    impl MediaSource for BlockingSource {
        fn read_at(
            &mut self,
            _: u64,
            _: &mut [u8],
            cancellation: &CancellationToken,
        ) -> Result<usize, PlayerError> {
            if cancellation.wait_cancelled(Duration::from_secs(2)) {
                Err(PlayerError::Stream("cancelled".into()))
            } else {
                Ok(0)
            }
        }
        fn size(&self) -> Option<u64> {
            Some(12)
        }
    }

    #[test]
    fn cancellation_wakes_blocked_source() {
        let token = CancellationToken::default();
        let worker_token = token.clone();
        let worker = thread::spawn(move || {
            let mut source = BlockingSource;
            source.read_at(0, &mut [0; 1], &worker_token).is_err()
        });
        thread::sleep(Duration::from_millis(20));
        token.cancel();
        assert!(worker.join().unwrap());
    }

    #[test]
    fn arbitrary_loader_path_is_not_an_option() {
        let paths = controlled_library_paths();
        assert!(paths.iter().all(|path| path.is_absolute()));
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with("opt/mpv/lib/libmpv.dylib"))
        );
    }

    #[test]
    fn player_loads_installed_libmpv_when_available() {
        if !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let player =
            Player::new(PlayerOptions::default()).expect("installed libmpv should initialize");
        assert!(player.decoder().is_ok());
    }

    struct MemoryFactory(Vec<u8>);
    struct MemorySource(Vec<u8>);
    impl MediaFactory for MemoryFactory {
        fn open(&self, _: &str) -> Result<Box<dyn MediaSource>, PlayerError> {
            Ok(Box::new(MemorySource(self.0.clone())))
        }
    }
    impl MediaSource for MemorySource {
        fn read_at(
            &mut self,
            offset: u64,
            buffer: &mut [u8],
            cancellation: &CancellationToken,
        ) -> Result<usize, PlayerError> {
            if cancellation.is_cancelled() {
                return Err(PlayerError::Stream("cancelled".into()));
            }
            let offset = offset as usize;
            if offset >= self.0.len() {
                return Ok(0);
            }
            let count = buffer.len().min(self.0.len() - offset);
            buffer[..count].copy_from_slice(&self.0[offset..offset + count]);
            Ok(count)
        }
        fn size(&self) -> Option<u64> {
            Some(self.0.len() as u64)
        }
    }

    #[test]
    fn libmpv_reads_seeks_and_lists_fixture_tracks_through_stream_cb() {
        let ffmpeg = Path::new("/opt/homebrew/bin/ffmpeg");
        if !ffmpeg.exists() || !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let subtitle = directory.path().join("fixture.srt");
        let video = directory.path().join("fixture.mp4");
        fs::write(
            &subtitle,
            "1\n00:00:00,000 --> 00:00:00,800\nFixture subtitle\n",
        )
        .unwrap();
        let status = Command::new(ffmpeg)
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=160x90:r=10:d=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:duration=1",
                "-i",
            ])
            .arg(&subtitle)
            .args([
                "-map", "0:v", "-map", "1:a", "-map", "2:s", "-c:v", "libx264", "-pix_fmt",
                "yuv420p", "-c:a", "aac", "-c:s", "mov_text",
            ])
            .arg(&video)
            .status()
            .unwrap();
        assert!(status.success());
        let mut options = PlayerOptions::default();
        options.extra.insert("vo".into(), "null".into());
        options.extra.insert("ao".into(), "null".into());
        let mut player = Player::new(options).unwrap();
        player
            .register_stream(
                "chill-memory",
                Arc::new(MemoryFactory(fs::read(video).unwrap())),
            )
            .unwrap();
        player.load("chill-memory://fixture").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut loaded = false;
        while Instant::now() < deadline {
            match player.poll_event(Duration::from_millis(100)) {
                PlayerEvent::FileLoaded => {
                    loaded = true;
                    break;
                }
                PlayerEvent::Ended { error, .. } if error < 0 => {
                    panic!("fixture failed with {error}")
                }
                _ => {}
            }
        }
        assert!(loaded, "libmpv did not load the stream callback fixture");
        player.pause(true).unwrap();
        player.seek_absolute(0.5).unwrap();
        let tracks = player.tracks().unwrap();
        assert!(tracks.iter().any(|track| track.kind == TrackKind::Audio));
        assert!(tracks.iter().any(|track| track.kind == TrackKind::Subtitle));
    }
}
