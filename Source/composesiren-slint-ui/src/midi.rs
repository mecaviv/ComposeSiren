//! The MIDI message a parameter value mirrors on the siren's channel, as the plugins send it to the real
//! sirens (CC numbers from `parameterDefinitions.h`, pitch bend as a 14-bit pitch wheel).

use crate::params::{Midi, ParamDef};

/// The message for `value` of `def` on a one-based `channel`, or None for UI-only parameters.
#[must_use]
pub fn message(def: &ParamDef, value: f32, channel: u8) -> Option<[u8; 3]> {
    let ch = channel.clamp(1, 16) - 1;
    match def.midi {
        Midi::Cc(cc) => Some([0xB0 | ch, cc, value.round().clamp(0.0, 127.0) as u8]),
        Midi::PitchWheel => {
            let v14 = (f32::midpoint(value.clamp(-1.0, 1.0), 1.0) * 16383.0).round() as u16;
            Some([0xE0 | ch, (v14 & 0x7f) as u8, (v14 >> 7) as u8])
        }
        Midi::None => None,
    }
}

/// A short human-readable form, for the editor's "MIDI out" line.
#[must_use]
pub fn describe(m: [u8; 3]) -> String {
    let ch = (m[0] & 0x0f) + 1;
    match m[0] & 0xf0 {
        0xB0 => format!("ch{ch} CC{} = {}", m[1], m[2]),
        0xE0 => format!(
            "ch{ch} pitch wheel = {}",
            u16::from(m[1]) | (u16::from(m[2]) << 7)
        ),
        0x90 => format!("ch{ch} note on {} vel {}", m[1], m[2]),
        0x80 => format!("ch{ch} note off {}", m[1]),
        _ => format!("{:02X} {:02X} {:02X}", m[0], m[1], m[2]),
    }
}
