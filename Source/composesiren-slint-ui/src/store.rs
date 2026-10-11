//! The parameter values shared by the host side (JUCE's `AudioProcessorValueTreeState`, the audio thread
//! reading them, automation) and the editor. Lock-free: one `AtomicU32` (f32 bits) per parameter and a
//! bitmask of values the host changed since the editor last looked, so the audio or message thread never
//! waits on the UI and the UI polls at frame rate.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::params::{PARAMS, ParamDef, ParamId};

/// The values of one strip.
pub struct ParamStore {
    values: [AtomicU32; PARAMS.len()],
    host_changes: AtomicU32,
}

impl Default for ParamStore {
    fn default() -> Self {
        Self {
            values: std::array::from_fn(|i| AtomicU32::new(PARAMS[i].default.to_bits())),
            host_changes: AtomicU32::new(0),
        }
    }
}

impl ParamStore {
    /// The current value of `id`.
    #[must_use]
    pub fn get(&self, id: ParamId) -> f32 {
        f32::from_bits(self.values[id as usize].load(Ordering::Relaxed))
    }

    /// A value set by the host (automation, a preset, a MIDI CC received): constrained, stored and flagged for
    /// the editor. Any thread. Returns the stored value.
    pub fn set_from_host(&self, id: ParamId, v: f32) -> f32 {
        let v = ParamDef::of(id).constrain(v);
        self.values[id as usize].store(v.to_bits(), Ordering::Relaxed);
        self.host_changes
            .fetch_or(1 << id as u32, Ordering::Release);
        v
    }

    /// A value set in the editor: constrained and stored, not flagged (the editor shows it already).
    /// Returns the stored value, which the caller forwards to the host.
    pub fn set_from_ui(&self, id: ParamId, v: f32) -> f32 {
        let v = ParamDef::of(id).constrain(v);
        self.values[id as usize].store(v.to_bits(), Ordering::Relaxed);
        v
    }

    /// The parameters the host changed since the last call, as a bitmask over `ParamId`; clears it.
    pub fn take_host_changes(&self) -> u32 {
        self.host_changes.swap(0, Ordering::Acquire)
    }
}

/// `ParamId` of an index into `PARAMS`.
#[must_use]
pub fn param_id(index: usize) -> Option<ParamId> {
    PARAMS.get(index).map(|d| d.id)
}
