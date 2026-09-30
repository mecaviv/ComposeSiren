//! Records ComposeSiren's audio output to FLAC or WAV.
//!
//! The audio thread hands each output block to [`Recorder::process`], which
//! only copies it into a lock-free ring buffer. A writer thread encodes and
//! writes the file: FLAC through `flacenc` ([`flac`]), WAV through `hound`,
//! both pure Rust, so the recorder works wherever the DSP runs, with or
//! without JUCE. [`ffi`] is the C ABI for the plugin.

pub mod ffi;
pub mod flac;
mod recorder;

pub use recorder::{Fade, Format, Recorder, Status, Summary};
