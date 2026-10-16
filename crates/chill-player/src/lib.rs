//! Dynamically loaded libmpv adapter.
//!
//! macOS does not support mpv's `wid` embedding. The AppKit layer must create
//! and make current an OpenGL context for its `NSView`, then call
//! [`Player::create_opengl_renderer`] and [`OpenGlRenderer::render`].

use chill_client::playback_cache::PlaybackCache;
use chill_client::{ClientError, Transport};
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use thiserror::Error;

const MPV_ERROR_LOADING_FAILED: i32 = -13;
const MAX_STREAM_READ_BYTES: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("libmpv introuvable dans les emplacements contrôlés")]
    LibraryNotFound,
    #[error("symbole libmpv absent: {0}")]
    MissingSymbol(String),
    #[error("API client libmpv {major}.{minor} trop ancienne (minimum 2.0)")]
    UnsupportedApi { major: u64, minor: u64 },
    #[error("libmpv a refusé l'opération ({0})")]
    Mpv(i32),
    #[error("valeur contenant un octet nul")]
    InvalidCString,
    #[error("option libmpv non autorisée: {0}")]
    UnsafeOption(String),
    #[error("réglage de tampon invalide: {0}")]
    InvalidCache(String),
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
    #[serde(default)]
    pub cache: PlaybackCache,
}

impl Default for PlayerOptions {
    fn default() -> Self {
        Self {
            native_window: None,
            hardware_decoding: true,
            initial_volume: 100,
            extra: BTreeMap::new(),
            cache: PlaybackCache::default(),
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
    #[serde(default)]
    pub original: bool,
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
    transport: Arc<RwLock<Transport>>,
    active: Arc<RwLock<Option<PlaybackStream>>>,
}

impl HttpMediaFactory {
    pub fn new(transport: Transport) -> Self {
        Self {
            transport: Arc::new(RwLock::new(transport)),
            active: Arc::new(RwLock::new(None)),
        }
    }
    pub fn set_playback(&self, playback: PlaybackStream) {
        *self.active.write().expect("playback lock poisoned") = Some(playback);
    }
    pub fn clear_playback(&self) {
        *self.active.write().expect("playback lock poisoned") = None;
    }
    /// Existing sources retain their own pinned transport/session snapshot.
    /// A server change applies only to a subsequent authorised stream open.
    pub fn set_transport(&self, transport: Transport) {
        *self.transport.write().expect("transport lock poisoned") = transport;
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
            worker: HttpReadWorker::start(
                self.transport
                    .read()
                    .expect("transport lock poisoned")
                    .clone(),
                active,
            )?,
            etag: None,
            size: None,
        }))
    }
}

struct HttpMediaSource {
    worker: HttpReadWorker,
    etag: Option<String>,
    size: Option<u64>,
}

enum HttpReadCommand {
    Read {
        offset: u64,
        length: usize,
        etag: Option<String>,
        response: mpsc::SyncSender<(Result<chill_client::RangeRead, ClientError>, Vec<u8>)>,
    },
    Shutdown,
}

struct HttpReadWorker {
    sender: mpsc::SyncSender<HttpReadCommand>,
    in_flight: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl HttpReadWorker {
    fn start(transport: Transport, playback: PlaybackStream) -> Result<Self, PlayerError> {
        Self::start_with_reader(move |offset, bytes, etag| {
            transport.read_media_range_blocking(
                &playback.media_id,
                &playback.session_id,
                playback.generation,
                offset,
                bytes,
                etag,
            )
        })
    }

    fn start_with_reader<F>(mut read: F) -> Result<Self, PlayerError>
    where
        F: FnMut(u64, &mut [u8], Option<&str>) -> Result<chill_client::RangeRead, ClientError>
            + Send
            + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(1);
        let in_flight = Arc::new(AtomicBool::new(false));
        let worker_in_flight = Arc::clone(&in_flight);
        let join = thread::Builder::new()
            .name("chill-media-read".into())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        HttpReadCommand::Read {
                            offset,
                            length,
                            etag,
                            response,
                        } => {
                            let mut bytes = vec![0; length];
                            let result = read(offset, &mut bytes, etag.as_deref());
                            // Network I/O is complete. Release the gate before
                            // publishing the response so a following read can
                            // safely queue even if this worker is not yet back
                            // in `recv`.
                            worker_in_flight.store(false, Ordering::Release);
                            let _ = response.send((result, bytes));
                        }
                        HttpReadCommand::Shutdown => break,
                    }
                }
            })
            .map_err(|error| PlayerError::Stream(error.to_string()))?;
        Ok(Self {
            sender,
            in_flight,
            join: Some(join),
        })
    }

    fn dispatch(&self, command: HttpReadCommand) -> Result<(), PlayerError> {
        try_dispatch_single_flight(&self.sender, &self.in_flight, command)
    }
}

fn try_dispatch_single_flight<T>(
    sender: &mpsc::SyncSender<T>,
    in_flight: &AtomicBool,
    command: T,
) -> Result<(), PlayerError> {
    if in_flight
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(PlayerError::Stream(
            "une lecture HTTP précédente est encore active".into(),
        ));
    }

    sender.try_send(command).map_err(|error| {
        in_flight.store(false, Ordering::Release);
        match error {
            mpsc::TrySendError::Full(_) => {
                PlayerError::Stream("file de lecture HTTP occupée".into())
            }
            mpsc::TrySendError::Disconnected(_) => {
                PlayerError::Stream("worker HTTP interrompu".into())
            }
        }
    })
}

impl Drop for HttpReadWorker {
    fn drop(&mut self) {
        let _ = self.sender.send(HttpReadCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
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
        // mpv's stream callback expects zero at EOF, not an HTTP 416 error.
        // Only a known, pinned size permits this local shortcut; unknown-size
        // errors and cancellation must remain failures.
        if buffer.is_empty() || self.size.is_some_and(|size| offset >= size) {
            return Ok(0);
        }
        // Exactly one request may be active or queued per open stream. A
        // cancelled reqwest request may live until Transport's timeout, but
        // repeated seeks cannot accumulate threads or request buffers.
        let length = capped_stream_read_length(buffer.len());
        let (sender, receiver) = mpsc::sync_channel(1);
        self.worker.dispatch(HttpReadCommand::Read {
            offset,
            length,
            etag: self.etag.clone(),
            response: sender,
        })?;
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
    // API 2.x layout from mpv 0.41 render_gl.h. Supporting 1.x would require
    // the removed trailing `extra_exts` field, so Player rejects API < 2.0.
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
    core: Rc<PlayerCore>,
    native_window: Option<NativeWindowHandle>,
}

struct PlayerCore {
    handle: NonNull<MpvHandle>,
    api: Api,
    _library: Library,
    registries: Mutex<Vec<usize>>,
}

impl Player {
    pub fn new(options: PlayerOptions) -> Result<Self, PlayerError> {
        options
            .cache
            .validate()
            .map_err(|e| PlayerError::InvalidCache(e.into()))?;
        let (library, _) = load_library()?;
        unsafe {
            let create: MpvCreate = symbol(&library, b"mpv_create\0")?;
            let api_version: MpvClientApiVersion = symbol(&library, b"mpv_client_api_version\0")?;
            let version = api_version();
            let (major, minor) = (version >> 16, version & 0xffff);
            if !supported_client_api(version) {
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
            let core = Rc::new(PlayerCore {
                handle,
                api,
                _library: library,
                registries: Mutex::new(Vec::new()),
            });
            let player = Self {
                core,
                native_window: options.native_window,
            };
            player.set_option("terminal", "no")?;
            // Only the Rust grant-bound callback may fetch media. Container/
            // playlist references and ambient mpv configuration are not inputs.
            for (name, value) in [
                ("config", "no"),
                ("load-scripts", "no"),
                ("ytdl", "no"),
                ("access-references", "no"),
                ("autoload-files", "no"),
                ("osc", "no"),
            ] {
                player.set_option(name, value)?;
            }
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
            // Packet cache is bounded across forward/backward playback. Decoded
            // frames and the webview are separate and measured in the client RSS.
            // The grant-bound custom scheme is not necessarily identified as
            // a network stream by mpv's cache=auto heuristic. Enable it
            // explicitly. 128/64 MiB cover approximately 60/30 seconds at
            // 18 Mbit/s; higher bitrates hit the byte limits earlier. mpv has
            // no time-based back-cache limit, so backward duration varies.
            player.set_option("cache", "yes")?;
            player.set_option(
                "demuxer-max-bytes",
                &format!("{}MiB", options.cache.forward_mib),
            )?;
            player.set_option(
                "demuxer-max-back-bytes",
                &format!("{}MiB", options.cache.back_mib),
            )?;
            player.set_option("cache-secs", "60")?;
            player.set_option("demuxer-readahead-secs", "60")?;
            player.set_option("demuxer-donate-buffer", "no")?;
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
            (self.core.api.stream_cb_add_ro)(
                self.core.handle.as_ptr(),
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
        self.core
            .registries
            .lock()
            .expect("registry lock poisoned")
            .push(raw as usize);
        Ok(())
    }

    pub fn load(&self, uri: &str) -> Result<(), PlayerError> {
        self.command(&["loadfile", uri, "replace"])
    }
    pub fn pause(&self, paused: bool) -> Result<(), PlayerError> {
        self.set_property("pause", if paused { "yes" } else { "no" })
    }
    pub fn is_buffering(&self) -> Result<bool, PlayerError> {
        Ok(property_bool(self.property("paused-for-cache")?))
    }

    /// Compressed packet-cache advance, measured by mpv (not the target).
    pub fn buffered_ahead_seconds(&self) -> Result<Option<f64>, PlayerError> {
        Ok(self
            .property("demuxer-cache-duration")?
            .and_then(|v| v.parse().ok()))
    }

    /// Apply device-local budgets while paused, immediately before a new load.
    pub fn set_cache_limits(&self, cache: PlaybackCache) -> Result<(), PlayerError> {
        cache
            .validate()
            .map_err(|e| PlayerError::InvalidCache(e.into()))?;
        self.set_property("demuxer-max-bytes", &format!("{}MiB", cache.forward_mib))?;
        self.set_property("demuxer-max-back-bytes", &format!("{}MiB", cache.back_mib))
    }
    pub fn set_volume(&self, volume: u8) -> Result<(), PlayerError> {
        self.set_property("volume", &volume.min(100).to_string())
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
                original: property_bool(self.property(&format!("{prefix}/original"))?),
            });
        }
        Ok(tracks)
    }

    pub fn poll_event(&self, timeout: Duration) -> PlayerEvent {
        let pointer =
            unsafe { (self.core.api.wait_event)(self.core.handle.as_ptr(), timeout.as_secs_f64()) };
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
            .core
            .api
            .render_context_create
            .ok_or(PlayerError::RenderApiUnavailable)?;
        let render = self
            .core
            .api
            .render_context_render
            .ok_or(PlayerError::RenderApiUnavailable)?;
        let free = self
            .core
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
        let result =
            unsafe { create(&mut context, self.core.handle.as_ptr(), params.as_mut_ptr()) };
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
            _core: self.core.clone(),
        })
    }

    fn set_option(&self, name: &str, value: &str) -> Result<(), PlayerError> {
        unsafe {
            call_string_pair(
                self.core.handle,
                self.core.api.set_option_string,
                name,
                value,
            )
        }
    }
    fn set_property(&self, name: &str, value: &str) -> Result<(), PlayerError> {
        unsafe {
            call_string_pair(
                self.core.handle,
                self.core.api.set_property_string,
                name,
                value,
            )
        }
    }
    fn property(&self, name: &str) -> Result<Option<String>, PlayerError> {
        let name = CString::new(name).map_err(|_| PlayerError::InvalidCString)?;
        let raw = unsafe {
            (self.core.api.get_property_string)(self.core.handle.as_ptr(), name.as_ptr())
        };
        if raw.is_null() {
            return Ok(None);
        }
        let value = unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() };
        unsafe { (self.core.api.free)(raw.cast()) };
        Ok(Some(value))
    }
    fn command(&self, parts: &[&str]) -> Result<(), PlayerError> {
        let strings = parts
            .iter()
            .map(|part| CString::new(*part).map_err(|_| PlayerError::InvalidCString))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = strings.iter().map(|part| part.as_ptr()).collect::<Vec<_>>();
        pointers.push(ptr::null());
        check(unsafe { (self.core.api.command)(self.core.handle.as_ptr(), pointers.as_ptr()) })
    }
}

impl Drop for PlayerCore {
    fn drop(&mut self) {
        unsafe {
            (self.api.terminate_destroy)(self.handle.as_ptr());
        }
        for registry in self
            .registries
            .get_mut()
            .expect("registry lock poisoned")
            .drain(..)
        {
            unsafe {
                drop(Arc::from_raw(registry as *const Registry));
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
    _core: Rc<PlayerCore>,
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
fn supported_client_api(version: u64) -> bool {
    version >> 16 >= 2
}
fn capped_stream_read_length(requested: usize) -> usize {
    requested.min(MAX_STREAM_READ_BYTES)
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

/// Read-only package check: no media, UI, client configuration or network.
pub fn verify_standalone_library() -> Result<(PathBuf, u64, u64), PlayerError> {
    let paths = controlled_library_paths();
    if paths.len() != 1 {
        return Err(PlayerError::LibraryNotFound);
    }
    let (library, path) = load_library()?;
    let version = unsafe {
        let get: MpvClientApiVersion = symbol(&library, b"mpv_client_api_version\0")?;
        get()
    };
    if !supported_client_api(version) {
        return Err(PlayerError::UnsupportedApi {
            major: version >> 16,
            minor: version & 0xffff,
        });
    }
    Ok((path, version >> 16, version & 0xffff))
}

fn controlled_library_paths() -> Vec<PathBuf> {
    library_paths_for_executable(std::env::current_exe().ok().as_deref())
}

fn library_paths_for_executable(executable: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(executable) = executable
        && let Some(bin) = executable.parent()
        && let Some(contents) = bin.parent()
        && bin.file_name().is_some_and(|name| name == "MacOS")
        && contents.file_name().is_some_and(|name| name == "Contents")
    {
        paths.push(contents.join("Frameworks/libmpv.dylib"));
        // A damaged standalone app must not silently depend on the build host.
        return paths;
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
    use std::sync::atomic::AtomicUsize;
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
    fn standalone_bundle_never_falls_back_to_homebrew() {
        assert_eq!(
            library_paths_for_executable(Some(Path::new(
                "/Apps/Chill Center.app/Contents/MacOS/chillcenter-desktop"
            ))),
            vec![PathBuf::from(
                "/Apps/Chill Center.app/Contents/Frameworks/libmpv.dylib"
            )]
        );
        assert!(
            library_paths_for_executable(Some(Path::new("/tmp/debug/chillcenter-desktop")))
                .iter()
                .any(|p| p.starts_with("/opt/homebrew"))
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

    #[test]
    fn installed_player_enables_bounded_one_minute_network_cache() {
        if !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let player = Player::new(PlayerOptions::default()).expect("libmpv initializes");
        assert_eq!(
            player.property("options/cache").unwrap().as_deref(),
            Some("yes")
        );
        for (name, expected) in [
            ("cache-secs", 60.),
            ("demuxer-readahead-secs", 60.),
            ("demuxer-max-bytes", 128. * 1024. * 1024.),
            ("demuxer-max-back-bytes", 64. * 1024. * 1024.),
        ] {
            let value = player
                .property(&format!("options/{name}"))
                .unwrap()
                .unwrap();
            assert_eq!(value.parse::<f64>().unwrap(), expected, "{name}");
        }
        assert_eq!(
            player
                .property("options/demuxer-donate-buffer")
                .unwrap()
                .as_deref(),
            Some("no")
        );
    }

    #[test]
    fn installed_player_uses_and_reapplies_device_cache_limits() {
        if !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let player = Player::new(PlayerOptions {
            cache: PlaybackCache {
                forward_mib: 96,
                back_mib: 32,
            },
            ..PlayerOptions::default()
        })
        .unwrap();
        assert_eq!(
            player
                .property("options/demuxer-max-bytes")
                .unwrap()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            96 * 1024 * 1024
        );
        player.pause(true).unwrap();
        player
            .set_cache_limits(PlaybackCache {
                forward_mib: 48,
                back_mib: 0,
            })
            .unwrap();
        assert_eq!(
            player
                .property("options/demuxer-max-bytes")
                .unwrap()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            48 * 1024 * 1024
        );
        assert_eq!(
            player
                .property("options/demuxer-max-back-bytes")
                .unwrap()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            0
        );
        assert!(
            player
                .set_cache_limits(PlaybackCache {
                    forward_mib: 513,
                    back_mib: 0
                })
                .is_err()
        );
        assert_eq!(
            player
                .property("options/demuxer-max-bytes")
                .unwrap()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            48 * 1024 * 1024
        );
    }

    #[test]
    fn client_api_requires_the_two_field_opengl_abi() {
        assert!(!supported_client_api((1 << 16) | 106));
        assert!(supported_client_api(2 << 16));
        assert!(supported_client_api((2 << 16) | 5));
    }

    #[test]
    fn stream_callback_request_allocation_is_capped() {
        assert_eq!(capped_stream_read_length(4096), 4096);
        assert_eq!(
            capped_stream_read_length(MAX_STREAM_READ_BYTES * 4),
            MAX_STREAM_READ_BYTES
        );
    }

    #[test]
    fn single_flight_dispatch_queues_before_receive_and_repeats() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let in_flight = AtomicBool::new(false);

        // The receiver is deliberately not waiting: this was rejected by the
        // former zero-capacity channel even though no read was active.
        try_dispatch_single_flight(&sender, &in_flight, 1).unwrap();
        assert!(try_dispatch_single_flight(&sender, &in_flight, 2).is_err());
        assert_eq!(receiver.recv().unwrap(), 1);

        // Model the worker completing network I/O, but not yet waiting in
        // recv again. A sequential request must still fit in the one-slot
        // channel.
        in_flight.store(false, Ordering::Release);
        try_dispatch_single_flight(&sender, &in_flight, 3).unwrap();
        assert_eq!(receiver.recv().unwrap(), 3);
    }

    #[test]
    fn http_worker_cancellation_preserves_backpressure_then_allows_reuse() {
        let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(0);
        let mut call = 0;
        let worker = HttpReadWorker::start_with_reader(move |_, bytes, _| {
            if call == 0 {
                entered_sender.send(()).unwrap();
                release_receiver.recv().unwrap();
            }
            call += 1;
            bytes[0] = call;
            Ok(chill_client::RangeRead {
                bytes_read: 1,
                total_size: Some(2),
                etag: Some("fixture".into()),
            })
        })
        .unwrap();
        let in_flight = Arc::clone(&worker.in_flight);
        let mut source = HttpMediaSource {
            worker,
            etag: None,
            size: None,
        };
        let cancellation = CancellationToken::default();
        let worker_cancellation = cancellation.clone();
        let read = thread::spawn(move || {
            let result = source.read_at(0, &mut [0; 1], &worker_cancellation);
            (result, source)
        });

        entered_receiver.recv().unwrap();
        cancellation.cancel();
        let (cancelled, mut source) = read.join().unwrap();
        assert!(cancelled.is_err());
        assert!(
            source
                .read_at(0, &mut [0; 1], &CancellationToken::default())
                .is_err(),
            "a blocked network read must retain the single-flight gate"
        );

        release_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while in_flight.load(Ordering::Acquire) && Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(!in_flight.load(Ordering::Acquire));
        let mut byte = [0];
        assert_eq!(
            source
                .read_at(1, &mut byte, &CancellationToken::default())
                .unwrap(),
            1
        );
        assert_eq!(byte, [2]);
    }

    #[test]
    fn http_worker_drop_joins_an_active_read() {
        let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(0);
        let worker = HttpReadWorker::start_with_reader(move |_, _, _| {
            entered_sender.send(()).unwrap();
            release_receiver.recv().unwrap();
            Ok(chill_client::RangeRead {
                bytes_read: 0,
                total_size: Some(0),
                etag: Some("fixture".into()),
            })
        })
        .unwrap();
        let (response_sender, _response_receiver) = mpsc::sync_channel(1);
        worker
            .dispatch(HttpReadCommand::Read {
                offset: 0,
                length: 1,
                etag: None,
                response: response_sender,
            })
            .unwrap();
        entered_receiver.recv().unwrap();

        let (dropped_sender, dropped_receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            drop(worker);
            dropped_sender.send(()).unwrap();
        });
        assert_eq!(
            dropped_receiver.recv_timeout(Duration::from_millis(20)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "drop must wait for the active read thread"
        );
        release_sender.send(()).unwrap();
        dropped_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker drop should join after the read completes");
    }

    #[test]
    fn known_http_eof_and_empty_buffers_do_not_issue_a_range_request() {
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = requests.clone();
        let worker = HttpReadWorker::start_with_reader(move |_, _, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            Err(ClientError::InvalidRangeResponse(
                "fixture: bytes beyond EOF return 416".into(),
            ))
        })
        .unwrap();
        let mut source = HttpMediaSource {
            worker,
            etag: Some("fixture".into()),
            size: Some(2),
        };
        for offset in [2, 3, u64::MAX] {
            assert_eq!(
                source
                    .read_at(offset, &mut [0; 1], &CancellationToken::default())
                    .unwrap(),
                0
            );
        }
        assert_eq!(
            source
                .read_at(0, &mut [], &CancellationToken::default())
                .unwrap(),
            0
        );
        assert_eq!(requests.load(Ordering::SeqCst), 0);
        let cancelled = CancellationToken::default();
        cancelled.cancel();
        assert!(source.read_at(2, &mut [0; 1], &cancelled).is_err());
        assert_eq!(requests.load(Ordering::SeqCst), 0);
        source.size = None;
        assert!(
            source
                .read_at(2, &mut [0; 1], &CancellationToken::default())
                .is_err()
        );
        assert_eq!(
            requests.load(Ordering::SeqCst),
            1,
            "unknown size must not hide an HTTP failure"
        );
    }

    static DROP_STAGE: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn fake_initialize(_: *mut MpvHandle) -> c_int {
        0
    }
    unsafe extern "C" fn fake_terminate(_: *mut MpvHandle) {
        assert_eq!(
            DROP_STAGE.load(Ordering::SeqCst),
            1,
            "core destroyed before renderer"
        );
        DROP_STAGE.store(2, Ordering::SeqCst);
    }
    unsafe extern "C" fn fake_set(_: *mut MpvHandle, _: *const c_char, _: *const c_char) -> c_int {
        0
    }
    unsafe extern "C" fn fake_get(_: *mut MpvHandle, _: *const c_char) -> *mut c_char {
        ptr::null_mut()
    }
    unsafe extern "C" fn fake_command(_: *mut MpvHandle, _: *const *const c_char) -> c_int {
        0
    }
    unsafe extern "C" fn fake_wait(_: *mut MpvHandle, _: c_double) -> *const MpvEvent {
        ptr::null()
    }
    unsafe extern "C" fn fake_free(_: *mut c_void) {}
    unsafe extern "C" fn fake_stream(
        _: *mut MpvHandle,
        _: *const c_char,
        _: *mut c_void,
        _: OpenFn,
    ) -> c_int {
        0
    }
    unsafe extern "C" fn fake_render(_: *mut MpvRenderContext, _: *mut MpvRenderParam) -> c_int {
        assert_eq!(
            DROP_STAGE.load(Ordering::SeqCst),
            0,
            "render used after core destruction"
        );
        0
    }
    unsafe extern "C" fn fake_render_free(_: *mut MpvRenderContext) {
        assert_eq!(
            DROP_STAGE.load(Ordering::SeqCst),
            0,
            "renderer freed after core destruction"
        );
        DROP_STAGE.store(1, Ordering::SeqCst);
    }

    #[test]
    fn renderer_keeps_core_and_library_alive_until_renderer_drop() {
        let Ok((library, _)) = load_library() else {
            return;
        };
        DROP_STAGE.store(0, Ordering::SeqCst);
        let core = Rc::new(PlayerCore {
            handle: NonNull::dangling(),
            api: Api {
                initialize: fake_initialize,
                terminate_destroy: fake_terminate,
                set_option_string: fake_set,
                set_property_string: fake_set,
                get_property_string: fake_get,
                command: fake_command,
                wait_event: fake_wait,
                free: fake_free,
                stream_cb_add_ro: fake_stream,
                render_context_create: None,
                render_context_render: Some(fake_render),
                render_context_free: Some(fake_render_free),
            },
            _library: library,
            registries: Mutex::new(Vec::new()),
        });
        let player = Player {
            core: core.clone(),
            native_window: None,
        };
        let resolver = Arc::into_raw(Arc::new(GlResolver {
            callback: Box::new(|_| ptr::null_mut()),
        }));
        let mut renderer = OpenGlRenderer {
            context: NonNull::dangling(),
            render: fake_render,
            free: fake_render_free,
            resolver,
            _core: core.clone(),
        };
        drop(core);
        drop(player);
        assert_eq!(DROP_STAGE.load(Ordering::SeqCst), 0);
        renderer
            .render(
                &mut OpenGlFramebuffer {
                    fbo: 0,
                    width: 1,
                    height: 1,
                    internal_format: 0,
                },
                false,
            )
            .unwrap();
        drop(renderer);
        assert_eq!(DROP_STAGE.load(Ordering::SeqCst), 2);
    }

    struct MemoryFactory(Vec<u8>);
    struct MemorySource(Vec<u8>);
    /// Deliberately throttled synthetic stream: it proves that libmpv keeps
    /// consuming its bounded packet cache after `pause=yes`, without touching
    /// a NAS, HTTP service or user media.
    struct SlowMemoryFactory {
        bytes: Vec<u8>,
        reads: Arc<AtomicUsize>,
    }
    struct SlowMemorySource {
        bytes: Vec<u8>,
        reads: Arc<AtomicUsize>,
    }
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
    impl MediaFactory for SlowMemoryFactory {
        fn open(&self, _: &str) -> Result<Box<dyn MediaSource>, PlayerError> {
            Ok(Box::new(SlowMemorySource {
                bytes: self.bytes.clone(),
                reads: self.reads.clone(),
            }))
        }
    }
    impl MediaSource for SlowMemorySource {
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
            if offset >= self.bytes.len() {
                return Ok(0);
            }
            thread::sleep(Duration::from_millis(12));
            self.reads.fetch_add(1, Ordering::Relaxed);
            let count = buffer.len().min(self.bytes.len() - offset);
            buffer[..count].copy_from_slice(&self.bytes[offset..offset + count]);
            Ok(count)
        }
        fn size(&self) -> Option<u64> {
            Some(self.bytes.len() as u64)
        }
    }

    #[test]
    fn libmpv_keeps_bounded_readahead_alive_while_paused_without_reloading() {
        let ffmpeg = Path::new("/opt/homebrew/bin/ffmpeg");
        if !ffmpeg.exists() || !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let video = directory.path().join("paused-cache.mp4");
        let status = Command::new(ffmpeg)
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=320x180:r=30:d=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=700:duration=30",
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-c:v",
                "libx264",
                "-b:v",
                "2M",
                "-c:a",
                "aac",
            ])
            .arg(&video)
            .status()
            .unwrap();
        assert!(status.success());
        let reads = Arc::new(AtomicUsize::new(0));
        let mut options = PlayerOptions::default();
        options.extra.insert("vo".into(), "null".into());
        options.extra.insert("ao".into(), "null".into());
        let mut player = Player::new(options).unwrap();
        player
            .register_stream(
                "chill-slow",
                Arc::new(SlowMemoryFactory {
                    bytes: fs::read(video).unwrap(),
                    reads: reads.clone(),
                }),
            )
            .unwrap();
        player.load("chill-slow://paused-cache").unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut loaded = false;
        while Instant::now() < deadline {
            if matches!(
                player.poll_event(Duration::from_millis(50)),
                PlayerEvent::FileLoaded
            ) {
                loaded = true;
                break;
            }
        }
        assert!(loaded, "libmpv did not load the synthetic stream");
        player.pause(true).unwrap();
        let before = reads.load(Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let _ = player.poll_event(Duration::from_millis(25));
        }
        let after = reads.load(Ordering::Relaxed);
        assert!(after > before, "paused libmpv stopped bounded readahead");
        let cached = player.buffered_ahead_seconds().unwrap().unwrap_or(0.0);
        assert!(
            (0.0..=60.1).contains(&cached),
            "cache duration escaped its 60s budget: {cached}"
        );
        // No second loadfile command is issued: the same Player instance is
        // simply unpaused, preserving its established source and packet cache.
        player.pause(false).unwrap();
        assert!(reads.load(Ordering::Relaxed) >= after);
    }

    #[test]
    fn media_references_cannot_open_unpinned_http() {
        assert_media_references_isolated(false);
    }
    #[test]
    fn hls_segments_cannot_open_unpinned_http() {
        assert_media_references_isolated(true);
    }
    fn assert_media_references_isolated(hls: bool) {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        if !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let contacted = Arc::new(AtomicBool::new(false));
        let observed = contacted.clone();
        let worker = thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(2);
            while Instant::now() < end {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        observed.store(true, Ordering::Release);
                        let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
                        let _ = socket.read(&mut [0; 1024]);
                        let _ = socket.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) => panic!("fixture listener: {e}"),
                }
            }
        });
        let mut options = PlayerOptions::default();
        options.extra.insert("vo".into(), "null".into());
        options.extra.insert("ao".into(), "null".into());
        let source = if hls {
            format!(
                "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-VERSION:3\n#EXT-X-MEDIA-SEQUENCE:0\n#EXTINF:1,\nhttp://{address}/must-not-open.ts\n#EXT-X-ENDLIST\n"
            )
        } else {
            format!("#EXTM3U\nhttp://{address}/must-not-open.mp4\n")
        };
        let mut player = Player::new(options).unwrap();
        player
            .register_stream("chill-memory", Arc::new(MemoryFactory(source.into_bytes())))
            .unwrap();
        player.load("chill-memory://reference-fixture.mp4").unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        while Instant::now() < end {
            let _ = player.poll_event(Duration::from_millis(20));
        }
        worker.join().unwrap();
        assert!(
            !contacted.load(Ordering::Acquire),
            "embedded media reference bypassed the pinned transport"
        );
    }
    #[test]
    fn transport_safety_options_cannot_be_overridden() {
        for name in [
            "config",
            "load-scripts",
            "ytdl",
            "access-references",
            "autoload-files",
            "demuxer-lavf-o",
            "scripts",
        ] {
            assert!(!allowed_extra_option(name));
        }
    }

    #[test]
    fn subtitle_changes_keep_the_current_playing_or_paused_state() {
        let ffmpeg = Path::new("/opt/homebrew/bin/ffmpeg");
        if !ffmpeg.exists() || !controlled_library_paths().iter().any(|path| path.exists()) {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let subtitle = directory.path().join("switch.srt");
        let video = directory.path().join("switch.mp4");
        fs::write(
            &subtitle,
            "1\n00:00:00,000 --> 00:00:04,000\nSubtitle fixture\n",
        )
        .unwrap();
        assert!(
            Command::new(ffmpeg)
                .args([
                    "-loglevel",
                    "error",
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    "color=c=black:s=160x90:r=10:d=4",
                    "-i"
                ])
                .arg(&subtitle)
                .args([
                    "-map", "0:v", "-map", "1:s", "-c:v", "libx264", "-c:s", "mov_text"
                ])
                .arg(&video)
                .status()
                .unwrap()
                .success()
        );
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
        player.load("chill-memory://subtitle-switch").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline, "fixture failed to load");
            if matches!(
                player.poll_event(Duration::from_millis(50)),
                PlayerEvent::FileLoaded
            ) {
                break;
            }
        }
        player.pause(false).unwrap();
        let track = player
            .tracks()
            .unwrap()
            .iter()
            .find(|t| t.kind == TrackKind::Subtitle)
            .unwrap()
            .id;
        for choice in [None, Some(track), None] {
            let before = player.position_seconds().unwrap().unwrap_or(0.0);
            player.select_subtitle(choice).unwrap();
            std::thread::sleep(Duration::from_millis(250));
            assert!(!property_bool(player.property("pause").unwrap()));
            assert!(
                player.position_seconds().unwrap().unwrap() > before + 0.1,
                "subtitle selection interrupted video time"
            );
        }
        let before = player.position_seconds().unwrap().unwrap();
        player
            .add_subtitle(
                subtitle.to_str().unwrap(),
                Some("External fixture"),
                Some("en"),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(250));
        assert!(!property_bool(player.property("pause").unwrap()));
        assert!(player.position_seconds().unwrap().unwrap() > before + 0.1);
        player.pause(true).unwrap();
        player.select_subtitle(Some(track)).unwrap();
        assert!(
            property_bool(player.property("pause").unwrap()),
            "changing subtitles must not resume an intentionally paused video"
        );
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
