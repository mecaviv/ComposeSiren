//! The C ABI. `build.rs` generates `include/clic_composesiren.h` from this
//! file with cbindgen: the doc comments here are the header's documentation.
//!
//! Every function accepts a null handle and does nothing with it. No function
//! panics across the ABI.

#![allow(unsafe_code, reason = "a C ABI takes raw pointers")]
#![allow(non_camel_case_types, reason = "C naming: snake_case_t")]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use crate::Clic;

/// Opaque handle to a click engine.
pub struct clic_t(Clic);

/// A new click engine with the built-in clicks at `sample_rate`, or null if
/// it cannot be created. Free it with `clic_free`.
#[unsafe(no_mangle)]
pub extern "C" fn clic_new(sample_rate: f64) -> *mut clic_t {
    catch_unwind(|| Box::into_raw(Box::new(clic_t(Clic::new(sample_rate))))).unwrap_or(ptr::null_mut())
}

/// Frees an engine from `clic_new`.
///
/// # Safety
///
/// `clic` is null or came from `clic_new` and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clic_free(clic: *mut clic_t) {
    if !clic.is_null() {
        // SAFETY: from clic_new, freed once
        drop(unsafe { Box::from_raw(clic) });
    }
}

/// Reloads the clicks at `sample_rate` (allocates: call it from
/// `prepareToPlay`, not from the audio thread).
///
/// # Safety
///
/// `clic` is null or a live handle from `clic_new`, not used concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clic_set_sample_rate(clic: *mut clic_t, sample_rate: f64) {
    // SAFETY: the caller's contract
    if let Some(c) = unsafe { clic.as_mut() } {
        let _ = catch_unwind(AssertUnwindSafe(|| c.0.set_sample_rate(sample_rate)));
    }
}

/// A MIDI message (`data2` is ignored for 2-byte messages). Channel 10:
/// note on with note > 1 and velocity > 1 plays the strong click (even note)
/// or the weak click (odd note); a program change picks the click.
///
/// # Safety
///
/// `clic` is null or a live handle from `clic_new`, not used concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clic_midi(clic: *mut clic_t, status: u8, data1: u8, data2: u8) {
    // SAFETY: the caller's contract
    if let Some(c) = unsafe { clic.as_mut() } {
        c.0.midi(status, data1, data2);
    }
}

/// Renders `frames` frames into `left` and `right` (overwritten).
///
/// # Safety
///
/// `clic` is null or a live handle from `clic_new`, not used concurrently;
/// `left` and `right` are null or point to `frames` writable floats each.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clic_render(clic: *mut clic_t, left: *mut f32, right: *mut f32, frames: usize) {
    // SAFETY: the caller's contract
    let Some(c) = (unsafe { clic.as_mut() }) else { return };
    if left.is_null() || right.is_null() || frames == 0 {
        return;
    }
    // SAFETY: frames writable floats each, per the contract
    let (l, r) = unsafe { (std::slice::from_raw_parts_mut(left, frames), std::slice::from_raw_parts_mut(right, frames)) };
    c.0.render(l, r);
}

/// The chosen click (0 = the first), or -1 for a null handle.
///
/// # Safety
///
/// `clic` is null or a live handle from `clic_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clic_current(clic: *const clic_t) -> i32 {
    // SAFETY: the caller's contract
    unsafe { clic.as_ref() }.map_or(-1, |c| i32::try_from(c.0.current()).unwrap_or(-1))
}

/// How many clicks there are, or 0 for a null handle.
///
/// # Safety
///
/// `clic` is null or a live handle from `clic_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clic_count(clic: *const clic_t) -> i32 {
    // SAFETY: the caller's contract
    unsafe { clic.as_ref() }.map_or(0, |c| i32::try_from(c.0.count()).unwrap_or(0))
}
