#![allow(deprecated)] // libmpv's supported macOS render API still uses OpenGL.

#[cfg(not(target_os = "macos"))]
compile_error!("chill-macos-render-probe only runs on macOS");

use chill_player::{OpenGlFramebuffer, Player, PlayerEvent, PlayerOptions};
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyClass;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSOpenGLContext, NSOpenGLPFAAccelerated, NSOpenGLPFAAlphaSize, NSOpenGLPFAColorSize,
    NSOpenGLPFAOpenGLProfile, NSOpenGLPixelFormat, NSOpenGLProfileVersion3_2Core,
};
use std::error::Error;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::{self, NonNull};
use std::thread;
use std::time::{Duration, Instant};

const GL_TEXTURE_2D: c_uint = 0x0DE1;
const GL_RGBA8: c_int = 0x8058;
const GL_RGBA: c_uint = 0x1908;
const GL_UNSIGNED_BYTE: c_uint = 0x1401;
const GL_TEXTURE_MIN_FILTER: c_uint = 0x2801;
const GL_TEXTURE_MAG_FILTER: c_uint = 0x2800;
const GL_LINEAR: c_int = 0x2601;
const GL_FRAMEBUFFER: c_uint = 0x8D40;
const GL_COLOR_ATTACHMENT0: c_uint = 0x8CE0;
const GL_FRAMEBUFFER_COMPLETE: c_uint = 0x8CD5;

#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {
    fn glGenFramebuffers(count: c_int, framebuffers: *mut c_uint);
    fn glBindFramebuffer(target: c_uint, framebuffer: c_uint);
    fn glDeleteFramebuffers(count: c_int, framebuffers: *const c_uint);
    fn glGenTextures(count: c_int, textures: *mut c_uint);
    fn glBindTexture(target: c_uint, texture: c_uint);
    fn glTexParameteri(target: c_uint, name: c_uint, value: c_int);
    fn glTexImage2D(
        target: c_uint,
        level: c_int,
        internal_format: c_int,
        width: c_int,
        height: c_int,
        border: c_int,
        format: c_uint,
        kind: c_uint,
        pixels: *const c_void,
    );
    fn glFramebufferTexture2D(
        target: c_uint,
        attachment: c_uint,
        texture_target: c_uint,
        texture: c_uint,
        level: c_int,
    );
    fn glCheckFramebufferStatus(target: c_uint) -> c_uint;
    fn glDeleteTextures(count: c_int, textures: *const c_uint);
    fn glReadPixels(
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
        format: c_uint,
        kind: c_uint,
        pixels: *mut c_void,
    );
    fn glFinish();
}

struct GlSymbols(usize);

impl GlSymbols {
    fn open() -> Result<Self, Box<dyn Error>> {
        let path = c"/System/Library/Frameworks/OpenGL.framework/OpenGL";
        let handle = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            return Err(format!("OpenGL.framework dlopen failed: {}", dynamic_error()).into());
        }
        Ok(Self(handle as usize))
    }

    fn resolve(&self, name: &CStr) -> *mut c_void {
        unsafe { libc::dlsym(self.0 as *mut c_void, name.as_ptr()) }
    }
}

impl Drop for GlSymbols {
    fn drop(&mut self) {
        unsafe { libc::dlclose(self.0 as *mut c_void) };
    }
}

fn dynamic_error() -> String {
    let error = unsafe { libc::dlerror() };
    if error.is_null() {
        "unknown dynamic-loader error".into()
    } else {
        unsafe { CStr::from_ptr(error.cast::<c_char>()) }
            .to_string_lossy()
            .into_owned()
    }
}

struct Framebuffer {
    id: c_uint,
    texture: c_uint,
    width: c_int,
    height: c_int,
}

impl Framebuffer {
    fn new(width: c_int, height: c_int) -> Result<Self, Box<dyn Error>> {
        let mut id = 0;
        let mut texture = 0;
        unsafe {
            glGenFramebuffers(1, &mut id);
            glBindFramebuffer(GL_FRAMEBUFFER, id);
            glGenTextures(1, &mut texture);
            glBindTexture(GL_TEXTURE_2D, texture);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                ptr::null(),
            );
            glFramebufferTexture2D(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                texture,
                0,
            );
            let status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
            if status != GL_FRAMEBUFFER_COMPLETE {
                glDeleteTextures(1, &texture);
                glDeleteFramebuffers(1, &id);
                return Err(format!("incomplete OpenGL framebuffer: 0x{status:x}").into());
            }
        }
        Ok(Self {
            id,
            texture,
            width,
            height,
        })
    }

    fn descriptor(&self) -> OpenGlFramebuffer {
        OpenGlFramebuffer {
            fbo: self.id as c_int,
            width: self.width,
            height: self.height,
            internal_format: GL_RGBA8,
        }
    }

    fn readback(&self) -> Vec<u8> {
        let mut pixels = vec![0; self.width as usize * self.height as usize * 4];
        unsafe {
            glBindFramebuffer(GL_FRAMEBUFFER, self.id);
            glFinish();
            glReadPixels(
                0,
                0,
                self.width,
                self.height,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                pixels.as_mut_ptr().cast(),
            );
        }
        pixels
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            glDeleteTextures(1, &self.texture);
            glDeleteFramebuffers(1, &self.id);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    autoreleasepool(|_| run())
}

fn run() -> Result<(), Box<dyn Error>> {
    let _main_thread = MainThreadMarker::new().ok_or("probe must run on the AppKit main thread")?;
    let _app_kit_loaded =
        AnyClass::get(c"NSApplication").ok_or("AppKit NSApplication class unavailable")?;
    let context = create_context()?;
    context.makeCurrentContext();
    let gl = GlSymbols::open()?;
    let fixture_directory = tempfile::tempdir()?;
    let fixture = create_fixture(fixture_directory.path())?;

    let mut options = PlayerOptions {
        hardware_decoding: false,
        ..PlayerOptions::default()
    };
    options.extra.insert("vo".into(), "libmpv".into());
    options.extra.insert("ao".into(), "null".into());
    let player = Player::new(options)?;
    let mut renderer = player.create_opengl_renderer(move |name| gl.resolve(name))?;
    player.load(fixture.to_str().ok_or("fixture path is not UTF-8")?)?;
    wait_for_loaded(&player)?;

    let first = render_until_nonblank(&player, &mut renderer, 320, 180)?;
    player.pause(true)?;
    player.seek_absolute(1.25)?;
    player.pause(false)?;
    let resized = render_until_nonblank(&player, &mut renderer, 640, 360)?;
    player.pause(true)?;

    println!("PROBE_OK libmpv=dynamic appkit_context=NSOpenGLContext main_thread=true");
    println!("FRAME_320x180 nonzero={} distinct={}", first.0, first.1);
    println!("FRAME_640x360 nonzero={} distinct={}", resized.0, resized.1);
    println!(
        "SEEK_PAUSE_RESIZE_OK position={:?}",
        player.position_seconds()?
    );

    drop(renderer);
    drop(player);
    NSOpenGLContext::clearCurrentContext();
    context.clearDrawable();
    Ok(())
}

fn create_context() -> Result<objc2::rc::Retained<NSOpenGLContext>, Box<dyn Error>> {
    let mut attributes = [
        NSOpenGLPFAAccelerated,
        NSOpenGLPFAOpenGLProfile,
        NSOpenGLProfileVersion3_2Core,
        NSOpenGLPFAColorSize,
        24,
        NSOpenGLPFAAlphaSize,
        8,
        0,
    ];
    let attributes = NonNull::new(attributes.as_mut_ptr()).ok_or("pixel attributes unavailable")?;
    let format = unsafe {
        NSOpenGLPixelFormat::initWithAttributes(NSOpenGLPixelFormat::alloc(), attributes)
    }
    .ok_or("NSOpenGLPixelFormat rejected the requested profile")?;
    NSOpenGLContext::initWithFormat_shareContext(NSOpenGLContext::alloc(), &format, None)
        .ok_or_else(|| "NSOpenGLContext creation failed".into())
}

fn create_fixture(directory: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = directory.join("render-probe.mp4");
    let status = Command::new("/opt/homebrew/bin/ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=30:duration=3",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&path)
        .status()?;
    if !status.success() {
        return Err("ffmpeg fixture generation failed".into());
    }
    Ok(path)
}

fn wait_for_loaded(player: &Player) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match player.poll_event(Duration::from_millis(50)) {
            PlayerEvent::FileLoaded => return Ok(()),
            PlayerEvent::Ended { error, .. } if error < 0 => {
                return Err(format!("libmpv ended fixture with error {error}").into());
            }
            _ => {}
        }
    }
    Err("timed out waiting for libmpv fixture".into())
}

fn render_until_nonblank(
    player: &Player,
    renderer: &mut chill_player::OpenGlRenderer,
    width: c_int,
    height: c_int,
) -> Result<(usize, usize), Box<dyn Error>> {
    let framebuffer = Framebuffer::new(width, height)?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        let mut descriptor = framebuffer.descriptor();
        renderer.render(&mut descriptor, true)?;
        let pixels = framebuffer.readback();
        let nonzero = pixels.iter().filter(|value| **value != 0).count();
        let mut seen = [false; 256];
        for value in &pixels {
            seen[*value as usize] = true;
        }
        let distinct = seen.into_iter().filter(|value| *value).count();
        if nonzero > pixels.len() / 20 && distinct > 8 {
            return Ok((nonzero, distinct));
        }
        if matches!(player.poll_event(Duration::from_millis(10)), PlayerEvent::Ended { error, .. } if error < 0)
        {
            return Err("libmpv playback failed before a rendered frame".into());
        }
        thread::sleep(Duration::from_millis(16));
    }
    Err(format!("no nonblank {width}x{height} frame before timeout").into())
}
