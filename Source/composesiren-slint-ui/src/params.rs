//! The siren parameters shown by the `OneSiren` strip, mirrored from
//! `ComposeSirenCore/lib/definitions/parameterDefinitions.h` (ids, code names, bounds, MIDI CC numbers).
//!
//! In this proof of concept the table is duplicated; a port would make one side the source of truth
//! (this Rust table exported through the C ABI, or a table generated from it for C++), see the doc.

/// The parameters of one siren strip, in display order.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamId {
    /// Pitch bend, -1..1, sent as a 14-bit pitch-wheel message.
    PitchBend,
    /// Pitch-bend range in semitones, CC16.
    PitchBendRange,
    /// Transpose in semitones (UI state, no CC).
    Transpose,
    /// Portamento, CC5.
    Portamento,
    /// Vibrato speed, CC9.
    VibratoFrequency,
    /// Vibrato depth, CC1.
    VibratoAmplitude,
    /// Vibrato "evolve" (acceleration), CC11.
    VibratoAcceleration,
    /// Tremolo speed, CC15.
    TremoloFrequency,
    /// Tremolo depth, CC92.
    TremoloAmplitude,
    /// Envelope attack, CC73.
    AttackDuration,
    /// Envelope release, CC72.
    ReleaseDuration,
    /// Timbre, CC13.
    Timbre,
    /// Mute, CC12.
    Mute,
    /// Volume, CC7.
    Volume,
}

/// How a parameter reaches the siren's MIDI output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Midi {
    /// A control change with this number (value 0..127).
    Cc(u8),
    /// The pitch wheel (14 bits).
    PitchWheel,
    /// Not sent: plugin-side state.
    None,
}

/// How the editor draws a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    /// A rotary knob filling from its minimum.
    Knob,
    /// A rotary knob filling from its centre (bipolar).
    CentredKnob,
    /// Up/down buttons with a value box.
    Spin,
}

/// One parameter's definition.
#[derive(Clone, Copy, Debug)]
pub struct ParamDef {
    /// Which parameter.
    pub id: ParamId,
    /// `ParameterIdGet::codeName`, the part after the group in the JUCE parameter id (`S5 | Volume`).
    pub code_name: &'static str,
    /// The knob's caption.
    pub label: &'static str,
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
    /// Step (0: continuous).
    pub step: f32,
    /// Default value (double-click resets to it).
    pub default: f32,
    /// MIDI mapping.
    pub midi: Midi,
    /// Drawing.
    pub widget: Widget,
}

const fn def(
    id: ParamId,
    code_name: &'static str,
    label: &'static str,
    (min, max, step, default): (f32, f32, f32, f32),
    midi: Midi,
    widget: Widget,
) -> ParamDef {
    ParamDef {
        id,
        code_name,
        label,
        min,
        max,
        step,
        default,
        midi,
        widget,
    }
}

/// The strip's parameters, in `ParamId` order.
pub const PARAMS: [ParamDef; 14] = [
    def(
        ParamId::PitchBend,
        "PitchBend",
        "Bend",
        (-1.0, 1.0, 0.0, 0.0),
        Midi::PitchWheel,
        Widget::CentredKnob,
    ),
    def(
        ParamId::PitchBendRange,
        "PitchBendRange",
        "Bend Range",
        (1.0, 36.0, 1.0, 1.0),
        Midi::Cc(16),
        Widget::Spin,
    ),
    def(
        ParamId::Transpose,
        "Transpose",
        "Transpose",
        (-24.0, 24.0, 1.0, 0.0),
        Midi::None,
        Widget::Spin,
    ),
    def(
        ParamId::Portamento,
        "Portamento",
        "Porta",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(5),
        Widget::Knob,
    ),
    def(
        ParamId::VibratoFrequency,
        "VibratoFrequency",
        "Speed",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(9),
        Widget::Knob,
    ),
    def(
        ParamId::VibratoAmplitude,
        "VibratoAmplitude",
        "Depth",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(1),
        Widget::Knob,
    ),
    def(
        ParamId::VibratoAcceleration,
        "VibratoAcceleration",
        "Evolve",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(11),
        Widget::Knob,
    ),
    def(
        ParamId::TremoloFrequency,
        "TremoloFrequency",
        "Speed",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(15),
        Widget::Knob,
    ),
    def(
        ParamId::TremoloAmplitude,
        "TremoloAmplitude",
        "Depth",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(92),
        Widget::Knob,
    ),
    def(
        ParamId::AttackDuration,
        "AttackDuration",
        "Attack",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(73),
        Widget::Knob,
    ),
    def(
        ParamId::ReleaseDuration,
        "ReleaseDuration",
        "Release",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(72),
        Widget::Knob,
    ),
    def(
        ParamId::Timbre,
        "Timbre",
        "Timbre",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(13),
        Widget::Knob,
    ),
    def(
        ParamId::Mute,
        "Mute",
        "Mute",
        (0.0, 127.0, 1.0, 0.0),
        Midi::Cc(12),
        Widget::Knob,
    ),
    def(
        ParamId::Volume,
        "Volume",
        "Volume",
        (0.0, 127.0, 1.0, 127.0),
        Midi::Cc(7),
        Widget::Knob,
    ),
];

/// The strip's groups: title and the range of `PARAMS` they hold.
pub const GROUPS: [(&str, usize, usize); 5] = [
    ("Pitch", 0, 4),
    ("Vibrato", 4, 3),
    ("Tremolo", 7, 2),
    ("Envelope", 9, 2),
    ("", 11, 3),
];

impl ParamDef {
    /// The definition of `id`.
    #[must_use]
    pub fn of(id: ParamId) -> &'static ParamDef {
        &PARAMS[id as usize]
    }

    /// The caption with the CC number on a second line, as `SirenStripComponent::appendCCNumber` does.
    #[must_use]
    pub fn caption(&self) -> String {
        match self.midi {
            Midi::Cc(cc) => format!("{}\ncc{cc}", self.label),
            _ => self.label.to_string(),
        }
    }

    /// Clamp to the range and snap to the step.
    #[must_use]
    pub fn constrain(&self, v: f32) -> f32 {
        let v = v.clamp(self.min, self.max);
        if self.step > 0.0 {
            (((v - self.min) / self.step).round() * self.step + self.min).clamp(self.min, self.max)
        } else {
            v
        }
    }

    /// 0..1, as JUCE's `RangedAudioParameter` (and a host) sees the value.
    #[must_use]
    pub fn normalise(&self, v: f32) -> f32 {
        (self.constrain(v) - self.min) / (self.max - self.min)
    }

    /// The value of a normalised 0..1 position.
    #[must_use]
    pub fn denormalise(&self, n: f32) -> f32 {
        self.constrain(self.min + n.clamp(0.0, 1.0) * (self.max - self.min))
    }

    /// The text under the knob: integers for stepped parameters, two decimals otherwise.
    #[must_use]
    pub fn format(&self, v: f32) -> String {
        if self.step >= 1.0 {
            format!("{}", v.round() as i32)
        } else {
            format!("{v:.2}")
        }
    }
}

/// The siren categories of the "Siren type" menu (`sirenCategory`), and the siren each one shows by default.
pub const CATEGORIES: [(&str, usize); 5] = [
    ("Alto", 0),
    ("Bass", 2),
    ("Tenor", 3),
    ("Soprano", 4),
    ("Piccolo", 6),
];

/// The strip colour of each siren S1..S7 (`sirenColourById`: an HSL ramp from dark to light blue, 8 steps,
/// assigned S3, S4, S1, S2, S5, S6, S7).
#[must_use]
pub fn siren_colour(siren: usize) -> (u8, u8, u8) {
    let order = [2, 3, 0, 1, 4, 5, 6]; // S1..S7 -> ramp index
    let (hs, ss, ls) = rgb_to_hsl(0x46, 0x50, 0xc8);
    let (he, se, le) = rgb_to_hsl(0x00, 0xb4, 0xc8);
    let t = order[siren.min(6)] as f32 / 7.0;
    hsl_to_rgb(hs + (he - hs) * t, ss + (se - ss) * t, ls + (le - ls) * t)
}

#[allow(clippy::many_single_char_names)] // the colour-space names
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = f32::midpoint(max, min);
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f32::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

#[allow(clippy::many_single_char_names)]
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let hue = |p: f32, q: f32, t: f32| {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let to8 = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
    (
        to8(hue(p, q, h + 1.0 / 3.0)),
        to8(hue(p, q, h)),
        to8(hue(p, q, h - 1.0 / 3.0)),
    )
}
