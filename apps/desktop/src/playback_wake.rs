//! Owned idle-display assertion, scoped to effective video playback only.
//! Does not override explicit sleep, lid closure or system power policy.
use std::ffi::{c_char, c_void};
use std::ptr;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        kind: *const c_void,
        level: u32,
        name: *const c_void,
        id: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(id: u32) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        text: *const c_char,
        encoding: u32,
    ) -> *const c_void;
    fn CFRelease(value: *const c_void);
}

#[derive(Default)]
pub(crate) struct PlaybackWake {
    id: Option<u32>,
}
impl PlaybackWake {
    pub(crate) fn set_playing(&mut self, playing: bool) {
        if !playing {
            if let Some(id) = self.id.take() {
                unsafe { IOPMAssertionRelease(id) };
            }
        } else if self.id.is_none() {
            // Both strings are immutable CF objects retained through the call.
            // IOKit copies the assertion data; release our temporary references.
            unsafe {
                let kind = CFStringCreateWithCString(
                    ptr::null(),
                    c"PreventUserIdleDisplaySleep".as_ptr(),
                    0x08000100, // kCFStringEncodingUTF8
                );
                let name = CFStringCreateWithCString(
                    ptr::null(),
                    c"Chill Center: video playback".as_ptr(),
                    0x08000100,
                );
                let mut id = 0;
                if !kind.is_null()
                    && !name.is_null()
                    && IOPMAssertionCreateWithName(kind, 255, name, &mut id) == 0
                {
                    self.id = Some(id);
                }
                if !kind.is_null() {
                    CFRelease(kind);
                }
                if !name.is_null() {
                    CFRelease(name);
                }
            }
        }
    }
}
impl Drop for PlaybackWake {
    fn drop(&mut self) {
        self.set_playing(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOPMAssertionCopyProperties(id: u32) -> *const c_void;
    }
    fn exists(id: u32) -> bool {
        let properties = unsafe { IOPMAssertionCopyProperties(id) };
        let found = !properties.is_null();
        if !properties.is_null() {
            unsafe { CFRelease(properties) };
        }
        found
    }
    #[test]
    fn real_macos_assertion_is_scoped_to_playback_and_released_on_drop() {
        let mut wake = PlaybackWake::default();
        wake.set_playing(false);
        assert!(wake.id.is_none());
        wake.set_playing(true);
        let first = wake.id.expect("macOS must grant the playback assertion");
        assert!(exists(first));
        wake.set_playing(true);
        assert_eq!(wake.id, Some(first));
        wake.set_playing(false);
        assert!(!exists(first));
        wake.set_playing(true);
        let second = wake.id.unwrap();
        assert!(exists(second));
        drop(wake);
        assert!(!exists(second));
    }
}
