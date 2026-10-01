//! The C ABI. `build.rs` generates `include/mecaviv_bridge.h` from this file
//! with cbindgen: the doc comments here are the header's documentation.
//!
//! Every function accepts a null handle and does nothing with it. Siren
//! numbers are 1 to 7; others are ignored. No function panics across the ABI.

#![allow(unsafe_code, reason = "a C ABI takes raw pointers")]
#![allow(non_camel_case_types, reason = "C naming: snake_case_t")]

use std::ffi::c_char;
use std::ptr;

use mecaviv_v1::SirenId;
use mecaviv_v1::keb::DriveState;

use crate::{Backend, Bridge, DaemonStats};

/// Number of sirens: siren numbers are 1 to `MECAVIV_BRIDGE_NUM_SIRENS`.
pub const MECAVIV_BRIDGE_NUM_SIRENS: u8 = 7;

/// Opaque handle to a bridge.
pub struct mecaviv_bridge_t(Bridge);

/// A siren's drive state.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum mecaviv_st_state_t {
    /// No reply from the drive, or the bridge is disabled.
    MECAVIV_ST_STATE_UNKNOWN = -1,
    /// Drive disabled (standby).
    MECAVIV_ST_STATE_OFF = 0,
    /// Drive enabled (ST on).
    MECAVIV_ST_STATE_ON = 1,
}

impl From<DriveState> for mecaviv_st_state_t {
    fn from(state: DriveState) -> Self {
        match state {
            DriveState::Unknown => Self::MECAVIV_ST_STATE_UNKNOWN,
            DriveState::Disabled => Self::MECAVIV_ST_STATE_OFF,
            DriveState::Enabled => Self::MECAVIV_ST_STATE_ON,
        }
    }
}

/// How the park is driven.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum mecaviv_bridge_backend_t {
    /// No transport: the handle is null, or enable failed to open one.
    MECAVIV_BRIDGE_BACKEND_NONE = 0,
    /// This process talks to the boards.
    MECAVIV_BRIDGE_BACKEND_IN_PROCESS = 1,
    /// `mecaviv-bridge-daemon` owns the hardware link.
    MECAVIV_BRIDGE_BACKEND_DAEMON = 2,
}

impl From<Backend> for mecaviv_bridge_backend_t {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::None => Self::MECAVIV_BRIDGE_BACKEND_NONE,
            Backend::InProcess => Self::MECAVIV_BRIDGE_BACKEND_IN_PROCESS,
            Backend::Daemon => Self::MECAVIV_BRIDGE_BACKEND_DAEMON,
        }
    }
}

/// # Safety
///
/// `bridge` is null or was returned by `mecaviv_bridge_new` and not freed.
unsafe fn get<'a>(bridge: *const mecaviv_bridge_t) -> Option<&'a Bridge> {
    // SAFETY: guaranteed by the caller.
    unsafe { bridge.as_ref() }.map(|bridge| &bridge.0)
}

/// Creates a disabled bridge to the park at its default addresses. Returns
/// null if its thread cannot be created.
#[unsafe(no_mangle)]
pub extern "C" fn mecaviv_bridge_new() -> *mut mecaviv_bridge_t {
    Bridge::new().map_or(ptr::null_mut(), |bridge| {
        Box::into_raw(Box::new(mecaviv_bridge_t(bridge)))
    })
}

/// Stops and frees a bridge.
///
/// # Safety
///
/// `bridge` is null or was returned by `mecaviv_bridge_new` and not freed.
/// No other call on it may be running or follow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_free(bridge: *mut mecaviv_bridge_t) {
    if !bridge.is_null() {
        // SAFETY: guaranteed by the caller.
        drop(unsafe { Box::from_raw(bridge) });
    }
}

/// Drives the park or not. Enabling sends the reset handshake to every siren;
/// disabling drops what is queued and forgets the drive states.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_set_enabled(
    bridge: *const mecaviv_bridge_t,
    enabled: bool,
) {
    // SAFETY: guaranteed by the caller.
    if let Some(bridge) = unsafe { get(bridge) } {
        bridge.set_enabled(enabled);
    }
}

/// Whether the park is driven. False for a null handle.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_is_enabled(bridge: *const mecaviv_bridge_t) -> bool {
    // SAFETY: guaranteed by the caller.
    unsafe { get(bridge) }.is_some_and(Bridge::is_enabled)
}

/// Queues a 3-byte MIDI message. Real-time safe: no lock, no allocation, no
/// socket. Returns false if disabled or the queue is full. Only note off, note
/// on, control change and pitch bend on channels 1 to 7 reach the sirens.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_push_midi(
    bridge: *const mecaviv_bridge_t,
    status: u8,
    data1: u8,
    data2: u8,
) -> bool {
    // SAFETY: guaranteed by the caller.
    unsafe { get(bridge) }.is_some_and(|bridge| bridge.push_midi([status, data1, data2]))
}

/// Resets siren `siren` (1 to 7): closes its flaps, stops its motor.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_reset(bridge: *const mecaviv_bridge_t, siren: u8) {
    // SAFETY: guaranteed by the caller.
    if let (Some(bridge), Some(siren)) = (unsafe { get(bridge) }, SirenId::new(siren)) {
        bridge.reset(siren);
    }
}

/// Resets every siren.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_reset_all(bridge: *const mecaviv_bridge_t) {
    // SAFETY: guaranteed by the caller.
    if let Some(bridge) = unsafe { get(bridge) } {
        bridge.reset_all();
    }
}

/// Enables (ST on) or disables every siren's drive.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_set_st_all(bridge: *const mecaviv_bridge_t, enabled: bool) {
    // SAFETY: guaranteed by the caller.
    if let Some(bridge) = unsafe { get(bridge) } {
        bridge.set_st_all(enabled);
    }
}

/// The drive state of siren `siren` (1 to 7), polled from its drive once per
/// second. Unknown for a null handle or another number.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_st_state(
    bridge: *const mecaviv_bridge_t,
    siren: u8,
) -> mecaviv_st_state_t {
    // SAFETY: guaranteed by the caller.
    match (unsafe { get(bridge) }, SirenId::new(siren)) {
        (Some(bridge), Some(siren)) => bridge.st_state(siren).into(),
        _ => mecaviv_st_state_t::MECAVIV_ST_STATE_UNKNOWN,
    }
}

/// How the park is driven. While disabled, a listening daemon socket is
/// reported as daemon so the UI can say what enabling will use. None for a
/// null handle.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_backend(
    bridge: *const mecaviv_bridge_t,
) -> mecaviv_bridge_backend_t {
    // SAFETY: guaranteed by the caller.
    unsafe { get(bridge) }.map_or(
        mecaviv_bridge_backend_t::MECAVIV_BRIDGE_BACKEND_NONE,
        |bridge| bridge.backend().into(),
    )
}

/// Tooltip for the current backend: a static NUL-terminated string. Do not
/// free it. The none tooltip for a null handle.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_backend_tooltip(
    bridge: *const mecaviv_bridge_t,
) -> *const c_char {
    // SAFETY: guaranteed by the caller.
    tooltip_ptr(unsafe { get(bridge) }.map_or(Backend::None, Bridge::backend))
}

/// Calls made to the daemon since the bridge was created. All zero when the
/// bridge never used the daemon.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct mecaviv_daemon_stats_t {
    /// Sessions opened.
    pub sessions: u64,
    /// Session attempts that failed.
    pub session_failures: u64,
    /// MIDI messages sent.
    pub midi: u64,
    /// MIDI messages dropped because the daemon does not carry them.
    pub midi_ignored: u64,
    /// Resets of one siren.
    pub resets: u64,
    /// Resets of every siren.
    pub reset_all: u64,
    /// ST of every siren.
    pub st_all: u64,
    /// Drive-state records received.
    pub drive_states: u64,
}

impl From<DaemonStats> for mecaviv_daemon_stats_t {
    fn from(s: DaemonStats) -> Self {
        Self {
            sessions: s.sessions,
            session_failures: s.session_failures,
            midi: s.midi,
            midi_ignored: s.midi_ignored,
            resets: s.resets,
            reset_all: s.reset_all,
            st_all: s.st_all,
            drive_states: s.drive_states,
        }
    }
}

/// Calls made to the daemon since the bridge was created. All zero for a
/// null handle.
///
/// # Safety
///
/// `bridge` is null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mecaviv_bridge_daemon_stats(
    bridge: *const mecaviv_bridge_t,
) -> mecaviv_daemon_stats_t {
    // SAFETY: guaranteed by the caller.
    unsafe { get(bridge) }.map_or_else(Default::default, |bridge| bridge.daemon_stats().into())
}

/// The library version, `major.minor.patch`, as a static NUL-terminated
/// string. Do not free it.
#[unsafe(no_mangle)]
pub extern "C" fn mecaviv_bridge_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

fn tooltip_ptr(backend: Backend) -> *const c_char {
    match backend {
        Backend::None => concat!("Park bridge is not available.", "\0")
            .as_ptr()
            .cast(),
        Backend::InProcess => concat!("Uses the in-process bridge.", "\0").as_ptr().cast(),
        Backend::Daemon => concat!("Uses the mecaviv-bridge daemon.", "\0")
            .as_ptr()
            .cast(),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    use super::tooltip_ptr;
    use crate::Backend;

    #[test]
    #[allow(unsafe_code, reason = "reads the C strings the ABI returns")]
    fn c_tooltips_match_backend() {
        for backend in [Backend::None, Backend::InProcess, Backend::Daemon] {
            // SAFETY: tooltip_ptr returns a static NUL-terminated string.
            let tip = unsafe { CStr::from_ptr(tooltip_ptr(backend)) };
            assert_eq!(tip.to_str(), Ok(backend.tooltip()));
        }
    }
}
