//! C ABI. `build.rs` generates `include/composesiren_record.h` from this file.

use std::ffi::{CStr, c_char, c_int};
use std::path::Path;

use std::time::Duration;

use crate::recorder::{Fade, Format, Recorder};

/// Opaque recorder.
#[allow(non_camel_case_types)]
pub struct cs_rec_t(Recorder);

/// Where a recording stands (see `cs_rec_status`).
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct cs_rec_status_t {
    /// 1 while recording.
    pub recording: c_int,
    /// 0 FLAC 24-bit, 1 WAV 24-bit, 2 WAV 32-bit float; -1 before any recording.
    pub format: c_int,
    /// The recording's sample rate, in Hz.
    pub sample_rate: u32,
    /// The recording's channels.
    pub channels: u32,
    /// Frames written to the file so far.
    pub frames_written: u64,
    /// Frames the audio thread could not hand over.
    pub frames_dropped: u64,
    /// 1 if the writer stopped on an error (see `cs_rec_error`).
    pub failed: c_int,
    /// 1 while a fading stop is under way (the recording ends by itself).
    pub fading: c_int,
}

/// A new recorder, not recording. Free it with `cs_rec_destroy`.
#[unsafe(no_mangle)]
pub extern "C" fn cs_rec_create() -> *mut cs_rec_t {
    Box::into_raw(Box::new(cs_rec_t(Recorder::new())))
}

/// Stops a running recording (completing its file) and frees the recorder.
///
/// # Safety
///
/// `rec` is null or from `cs_rec_create`, not yet destroyed, and no other
/// call on it is running or follows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_destroy(rec: *mut cs_rec_t) {
    if !rec.is_null() {
        // SAFETY: from `cs_rec_create`, as the caller guarantees.
        drop(unsafe { Box::from_raw(rec) });
    }
}

/// The audio thread's output block: `channels` pointers to `frames` samples.
/// Copies it when recording. Allocates nothing, takes no lock. Call it from one
/// thread at a time (the audio thread).
///
/// # Safety
///
/// `rec` is valid; `data` holds `channels` pointers, each to `frames` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_process(
    rec: *const cs_rec_t,
    data: *const *const f32,
    channels: u32,
    frames: u32,
) {
    const MAX_CHANNELS: usize = 16;
    if rec.is_null() || data.is_null() || channels == 0 || channels as usize > MAX_CHANNELS {
        return;
    }
    let mut slices: [&[f32]; MAX_CHANNELS] = [&[]; MAX_CHANNELS];
    for (c, slot) in slices.iter_mut().enumerate().take(channels as usize) {
        // SAFETY: `channels` valid pointers to `frames` floats, as the caller guarantees.
        let ptr = unsafe { *data.add(c) };
        if ptr.is_null() {
            return;
        }
        // SAFETY: as above.
        *slot = unsafe { std::slice::from_raw_parts(ptr, frames as usize) };
    }
    // SAFETY: valid, as the caller guarantees.
    unsafe { &*rec }.0.process(&slices[..channels as usize]);
}

fn write_message(message: &str, out: *mut c_char, len: u32) {
    if out.is_null() || len == 0 {
        return;
    }
    let bytes = message.as_bytes();
    let n = bytes.len().min(len as usize - 1);
    // SAFETY: `out` has room for `len` bytes (the callers' contract).
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), n);
        *out.add(n) = 0;
    }
}

/// Starts recording to `path` (UTF-8, null-terminated) in `format` (0 FLAC
/// 24-bit, 1 WAV 24-bit, 2 WAV 32-bit float), for blocks of `channels`
/// channels at `sample_rate`. Returns 0, or -1 with the reason in `error`.
///
/// # Safety
///
/// `rec` is valid; `path` is null-terminated; `error` has room for
/// `error_len` bytes, or is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_start(
    rec: *const cs_rec_t,
    path: *const c_char,
    format: u32,
    sample_rate: u32,
    channels: u32,
    error: *mut c_char,
    error_len: u32,
) -> c_int {
    if rec.is_null() || path.is_null() {
        write_message("no recorder or no path", error, error_len);
        return -1;
    }
    let Some(format) = Format::from_u32(format) else {
        write_message("unknown format", error, error_len);
        return -1;
    };
    // SAFETY: null-terminated, as the caller guarantees.
    let path = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: valid, as the caller guarantees.
    match unsafe { &*rec }
        .0
        .start(Path::new(&path), format, sample_rate, channels)
    {
        Ok(()) => 0,
        Err(e) => {
            write_message(&e, error, error_len);
            -1
        }
    }
}

/// Stops recording and completes the file. Returns 0, or -1 with the reason
/// in `error` (nothing was recording, or the writer failed).
///
/// # Safety
///
/// As for `cs_rec_start`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_stop(
    rec: *const cs_rec_t,
    error: *mut c_char,
    error_len: u32,
) -> c_int {
    if rec.is_null() {
        return -1;
    }
    // SAFETY: valid, as the caller guarantees.
    match unsafe { &*rec }.0.stop() {
        Ok(_) => 0,
        Err(e) => {
            write_message(&e, error, error_len);
            -1
        }
    }
}

/// Asks the recording to end: it goes on for up to `wait_ms`, ending as soon
/// as the sound has died out; if it still sounds then, it fades out over
/// `fade_ms` and ends. Returns at once (0, or -1 with the reason in `error`);
/// `cs_rec_status` says when it has ended.
///
/// # Safety
///
/// As for `cs_rec_start`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_stop_fading(
    rec: *const cs_rec_t,
    wait_ms: u32,
    fade_ms: u32,
    error: *mut c_char,
    error_len: u32,
) -> c_int {
    if rec.is_null() {
        return -1;
    }
    let fade = Fade {
        wait: Duration::from_millis(u64::from(wait_ms)),
        length: Duration::from_millis(u64::from(fade_ms)),
    };
    // SAFETY: valid, as the caller guarantees.
    match unsafe { &*rec }.0.stop_fading(fade) {
        Ok(()) => 0,
        Err(e) => {
            write_message(&e, error, error_len);
            -1
        }
    }
}

/// The running recording, or the last one.
///
/// # Safety
///
/// `rec` is valid and `out` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_status(rec: *const cs_rec_t, out: *mut cs_rec_status_t) {
    if rec.is_null() || out.is_null() {
        return;
    }
    // SAFETY: valid, as the caller guarantees.
    let s = unsafe { &*rec }.0.status();
    // SAFETY: writable, as the caller guarantees.
    unsafe {
        *out = cs_rec_status_t {
            recording: c_int::from(s.recording),
            format: s.format.map_or(-1, |f| f as c_int),
            sample_rate: s.sample_rate,
            channels: s.channels,
            frames_written: s.frames_written,
            frames_dropped: s.frames_dropped,
            failed: c_int::from(s.error.is_some()),
            fading: c_int::from(s.fading),
        };
    }
}

/// The running or last recording's file path, null-terminated in `out`.
/// Returns its length (0 before any recording).
///
/// # Safety
///
/// `rec` is valid; `out` has room for `len` bytes, or is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_path(rec: *const cs_rec_t, out: *mut c_char, len: u32) -> u32 {
    if rec.is_null() {
        return 0;
    }
    // SAFETY: valid, as the caller guarantees.
    let path = unsafe { &*rec }
        .0
        .status()
        .path
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    write_message(&path, out, len);
    path.len() as u32
}

/// The writer's last error, null-terminated in `out`. Returns its length.
///
/// # Safety
///
/// As for `cs_rec_path`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_rec_error(rec: *const cs_rec_t, out: *mut c_char, len: u32) -> u32 {
    if rec.is_null() {
        return 0;
    }
    // SAFETY: valid, as the caller guarantees.
    let error = unsafe { &*rec }.0.status().error.unwrap_or_default();
    write_message(&error, out, len);
    error.len() as u32
}
