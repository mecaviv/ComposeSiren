//! Proof of concept: ComposeSiren's `OneSiren` editor in Slint, Rust first.
//!
//! - `params`: the siren parameters (ids, bounds, CC numbers), mirrored from `parameterDefinitions.h`.
//! - `store`: lock-free values shared by the host side and the editor.
//! - `midi`: the MIDI message each value mirrors.
//! - `editor`: the Slint component bound to the store; UI changes go to a [`editor::HostSink`].
//! - `embed`: the editor rendered by Slint's software renderer into pixels the host owns, with the host's
//!   mouse events forwarded: how it sits inside a JUCE `AudioProcessorEditor` without a second window.
//! - `ffi`: the C ABI over `embed` for the JUCE plugin (`include/composesiren_slint_ui.h`).

pub mod editor;
pub mod embed;
pub mod ffi;
pub mod midi;
pub mod params;
pub mod store;

#[allow(missing_docs, clippy::all, clippy::pedantic)]
mod generated {
    slint::include_modules!();
}

pub use generated::{GroupRow, OneSiren, ParamRow};
