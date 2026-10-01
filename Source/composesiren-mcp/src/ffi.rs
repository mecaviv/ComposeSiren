//! C ABI. `build.rs` generates `include/composesiren_mcp.h` from this file.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ptr;

use crate::dispatch::{Dispatch, DispatchFn};
use crate::server::Running;

/// Opaque server. Null means it failed to start.
#[allow(non_camel_case_types, dead_code)]
pub struct cs_mcp_server_t(Running);

/// Plugin callback. `request_json` is one JSON object. The return value is a
/// `malloc`'d JSON object; the server frees it with `free`.
#[allow(non_camel_case_types)]
pub type cs_mcp_dispatch_fn = DispatchFn;

/// Start the server. `out_port` receives the bound port. Returns null on failure.
///
/// # Safety
///
/// `plugin_name` and `plugin_4cc` are null-terminated UTF-8 and live for the
/// call. `dispatch` and `user` stay valid until `cs_mcp_stop`. `out_port` is
/// writable. `standalone` is 0 or 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_mcp_start(
    plugin_name: *const c_char,
    plugin_4cc: *const c_char,
    standalone: c_int,
    dispatch: cs_mcp_dispatch_fn,
    user: *mut c_void,
    out_port: *mut c_int,
) -> *mut cs_mcp_server_t {
    // SAFETY: the plugin passes the callback it will keep alive.
    // SAFETY: both strings are null or null-terminated for this call, and the
    // callback stays valid until `cs_mcp_stop`.
    let dispatch = unsafe { Dispatch::new(dispatch, user) };
    let name = unsafe { c_str_or_empty(plugin_name) };
    let code = unsafe { c_str_or_empty(plugin_4cc) };
    match Running::start(&name, &code, standalone != 0, dispatch) {
        Ok(running) => {
            if !out_port.is_null() {
                // SAFETY: the caller passed a writable int.
                unsafe { *out_port = i32::from(running.port()) };
            }
            Box::into_raw(Box::new(cs_mcp_server_t(running)))
        }
        Err(err) => {
            eprintln!("composesiren-mcp failed to start: {err}");
            ptr::null_mut()
        }
    }
}

/// Stop the server and remove its discovery entry.
///
/// # Safety
///
/// `server` is null or was returned by `cs_mcp_start` and not stopped. No other
/// call on it follows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_mcp_stop(server: *mut cs_mcp_server_t) {
    if !server.is_null() {
        // SAFETY: the caller owns this pointer.
        drop(unsafe { Box::from_raw(server) });
    }
}

/// Per-tool call counts since the server started, as a null-terminated JSON
/// object `{"tools": {"<tool>": {"calls": n, "errors": n}}}`. Free the string
/// with `cs_mcp_free_string`. Returns null if `server` is null.
///
/// # Safety
///
/// `server` is null or was returned by `cs_mcp_start` and not stopped.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_mcp_stats_json(server: *const cs_mcp_server_t) -> *mut c_char {
    // SAFETY: guaranteed by the caller.
    let Some(server) = (unsafe { server.as_ref() }) else {
        return ptr::null_mut();
    };
    CString::new(server.0.stats().to_json().to_string()).map_or(ptr::null_mut(), CString::into_raw)
}

/// Free a string returned by `cs_mcp_stats_json`.
///
/// # Safety
///
/// `text` is null or came from `cs_mcp_stats_json` and was not freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_mcp_free_string(text: *mut c_char) {
    if !text.is_null() {
        // SAFETY: the caller got it from `CString::into_raw`.
        drop(unsafe { CString::from_raw(text) });
    }
}

/// # Safety
///
/// `text` is null or a null-terminated UTF-8 string.
unsafe fn c_str_or_empty(text: *const c_char) -> String {
    if text.is_null() {
        String::new()
    } else {
        // SAFETY: guaranteed by the caller.
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    }
}
