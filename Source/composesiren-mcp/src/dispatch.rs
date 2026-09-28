//! The call from a tool into the plugin. The plugin does the work on the
//! JUCE message thread and returns a JSON object.

use std::ffi::{CStr, CString, c_char, c_void};
use std::os::raw::c_void as RawVoid;

use serde_json::{Value, json};

/// C function implemented by the plugin.
///
/// `request_json` is a null-terminated UTF-8 JSON object. The return value is
/// a `malloc`'d null-terminated UTF-8 JSON object, which Rust frees.
pub type DispatchFn = unsafe extern "C" fn(*const c_char, *mut c_void) -> *mut c_char;

/// Shared handle to the plugin callback.
#[derive(Clone, Copy)]
pub struct Dispatch {
    function: DispatchFn,
    user: *mut c_void,
}

// The plugin synchronises `user`. The pointer does not move for the server's life.
unsafe impl Send for Dispatch {}
unsafe impl Sync for Dispatch {}

impl Dispatch {
    /// # Safety
    ///
    /// `function` and `user` stay valid until the server has stopped, and
    /// `function` is safe to call from a thread that is not the audio thread.
    pub unsafe fn new(function: DispatchFn, user: *mut c_void) -> Self {
        Self { function, user }
    }

    /// Send one JSON command and return the plugin's JSON reply.
    pub fn call(&self, request: Value) -> Value {
        let Ok(text) = serde_json::to_string(&request) else {
            return json!({"ok": false, "error": "could not encode the request"});
        };
        let Ok(c_request) = CString::new(text) else {
            return json!({"ok": false, "error": "the request contains a null"});
        };
        // SAFETY: the plugin promised the callback is valid for the server's life.
        let response = unsafe { (self.function)(c_request.as_ptr(), self.user) };
        if response.is_null() {
            return json!({"ok": false, "error": "the plugin returned no response"});
        }
        // SAFETY: the plugin returns a malloc'd null-terminated string.
        let owned = unsafe { CStr::from_ptr(response) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: the same malloc that produced `response`.
        unsafe { libc::free(response.cast::<RawVoid>()) };
        serde_json::from_str(&owned).unwrap_or_else(|_| json!({"ok": false, "error": owned}))
    }
}
