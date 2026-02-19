use arboard::Clipboard;

// ── macOS focus management via ObjC runtime (no extra deps, no permissions needed) ──

#[cfg(target_os = "macos")]
mod macos_focus {
    use std::ffi::c_void;

    extern "C" {
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
    }

    /// Get the PID of the currently frontmost application.
    pub fn get_frontmost_pid() -> i32 {
        unsafe {
            type SendObj = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
            type SendI32 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
            let send_obj: SendObj = std::mem::transmute(objc_msgSend as *const ());
            let send_i32: SendI32 = std::mem::transmute(objc_msgSend as *const ());

            let cls = objc_getClass(b"NSWorkspace\0".as_ptr());
            let workspace = send_obj(cls, sel_registerName(b"sharedWorkspace\0".as_ptr()));
            let app = send_obj(
                workspace,
                sel_registerName(b"frontmostApplication\0".as_ptr()),
            );
            if app.is_null() {
                return -1;
            }
            send_i32(app, sel_registerName(b"processIdentifier\0".as_ptr()))
        }
    }

    /// Bring an application to the foreground by its PID.
    pub fn activate_pid(pid: i32) {
        unsafe {
            type SendWithI32 = unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> *mut c_void;
            type SendWithUsize =
                unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> bool;
            let send_with_i32: SendWithI32 = std::mem::transmute(objc_msgSend as *const ());
            let send_with_usize: SendWithUsize = std::mem::transmute(objc_msgSend as *const ());

            let cls = objc_getClass(b"NSRunningApplication\0".as_ptr());
            let app = send_with_i32(
                cls,
                sel_registerName(b"runningApplicationWithProcessIdentifier:\0".as_ptr()),
                pid,
            );
            if !app.is_null() {
                // NSApplicationActivateIgnoringOtherApps = 2
                send_with_usize(app, sel_registerName(b"activateWithOptions:\0".as_ptr()), 2);
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos_focus::{activate_pid, get_frontmost_pid};

#[cfg(not(target_os = "macos"))]
pub fn get_frontmost_pid() -> i32 { -1 }

#[cfg(not(target_os = "macos"))]
pub fn activate_pid(_pid: i32) {}

pub fn paste_text(text: &str) -> Result<(), String> {
    // Save current clipboard content (best effort)
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Clipboard init error: {}", e))?;

    let previous = clipboard.get_text().ok();

    // Set new text
    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard set error: {}", e))?;

    // Small delay to ensure clipboard is ready
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Simulate Cmd+V / Ctrl+V
    simulate_paste()?;

    // Small delay then restore clipboard (best effort)
    std::thread::sleep(std::time::Duration::from_millis(100));
    if let Some(prev) = previous {
        let _ = clipboard.set_text(&prev);
    }

    Ok(())
}

/// Check (and optionally prompt for) macOS Accessibility permission.
/// Returns true if the app is already trusted.
#[cfg(target_os = "macos")]
pub fn ensure_accessibility_permission() -> bool {
    use std::ffi::c_void;

    #[repr(C)]
    struct CFDictionaryKeyCallBacks {
        _opaque: [u8; 0],
    }
    #[repr(C)]
    struct CFDictionaryValueCallBacks {
        _opaque: [u8; 0],
    }

    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;

        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const CFDictionaryKeyCallBacks,
            value_callbacks: *const CFDictionaryValueCallBacks,
        ) -> *const c_void;
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            c_str: *const i8,
            encoding: u32,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);

        static kCFBooleanTrue: *const c_void;
        static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
        static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
    }

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

    unsafe {
        let key = CFStringCreateWithCString(
            std::ptr::null(),
            b"AXTrustedCheckOptionPrompt\0".as_ptr() as *const i8,
            K_CF_STRING_ENCODING_UTF8,
        );

        let keys = [key];
        let values = [kCFBooleanTrue];

        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr() as *const *const c_void,
            values.as_ptr() as *const *const c_void,
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );

        let trusted = AXIsProcessTrustedWithOptions(dict);

        CFRelease(dict);
        CFRelease(key);

        trusted
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_accessibility_permission() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn simulate_paste() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // Key code 9 = 'v' on macOS
    const KEY_V: CGKeyCode = 9;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Failed to create CGEventSource")?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
        .map_err(|_| "Failed to create key down event")?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);

    let key_up = CGEvent::new_keyboard_event(source, KEY_V, false)
        .map_err(|_| "Failed to create key up event")?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);

    key_down.post(core_graphics::event::CGEventTapLocation::HID);
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init error: {}", e))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Key press error: {}", e))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Key click error: {}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Key release error: {}", e))?;

    Ok(())
}
