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

use crate::Bridge;

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

/// # Safety
///
/// `bridge` is null or was returned by `mecaviv_bridge_new` and not freed.
unsafe fn get<'a>(bridge: *const mecaviv_bridge_t) -> Option<&'a Bridge> {
    // SAFETY: guaranteed by the caller.
    unsafe { bridge.as_ref() }.map(|bridge| &bridge.0)
}

/// Creates a disabled bridge to the park at its default addresses. Returns
/// null if its sockets or its thread cannot be created.
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

/// The library version, `major.minor.patch`, as a static NUL-terminated
/// string. Do not free it.
#[unsafe(no_mangle)]
pub extern "C" fn mecaviv_bridge_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}
