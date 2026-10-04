//! GENERATED from the shared metadata (composesiren-parameters.csv, composesiren-ui-sections.csv, composesiren-ui-sirens.csv and composesiren-ui-theme.csv). Do not edit: change the metadata and regenerate.

/// `ParameterIdGet::getClass`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterClass {
    /// A siren strip parameter.
    Siren,
    /// The reverb.
    Reverb,
    /// A mixer track.
    Track,
    /// The master bus.
    Master,
    /// The click (an optional build).
    Clic,
}

/// How a parameter reaches the MIDI output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Midi {
    /// Not sent.
    None,
    /// A control change with this number.
    Cc(u8),
    /// The 14-bit pitch wheel.
    PitchWheel,
}

/// How an editor draws a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    /// Not drawn.
    None,
    /// A rotary knob filling from its minimum.
    Knob,
    /// A rotary knob filling from its centre.
    CentredKnob,
    /// Up/down buttons with a value box.
    Spin,
    /// An on/off switch.
    Toggle,
    /// A momentary button.
    Button,
}

/// One plugin parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameter {
    /// The JUCE parameter id is `<group> | <code_name>`.
    pub code_name: &'static str,
    /// Its class.
    pub class: ParameterClass,
    /// What hosts show.
    pub label: &'static str,
    /// The caption on the strip, empty when on no section.
    pub strip_label: &'static str,
    /// Unit shown after the value.
    pub unit: &'static str,
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
    /// Step, 0 for continuous.
    pub step: f32,
    /// Default value.
    pub default: f32,
    /// MIDI mapping.
    pub midi: Midi,
    /// Drawing.
    pub widget: Widget,
    /// Index in [`SECTIONS`] and place in that section.
    pub section: Option<(usize, usize)>,
    /// The build option the parameter needs, `None` when always built.
    pub required_option: Option<&'static str>,
}

/// A titled part of a strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// Identifier.
    pub id: &'static str,
    /// The class of its parameters.
    pub class: ParameterClass,
    /// Title, may be empty.
    pub title: &'static str,
}

/// A siren as the editors show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SirenUi {
    /// `S1` … `S7`.
    pub id: &'static str,
    /// 0xAARRGGBB.
    pub strip_colour: u32,
    /// The siren the Siren type menu shows for its category.
    pub category_default: bool,
}

/// The parameters, in host order.
pub const PARAMETERS: [Parameter; 32] = [
    Parameter {
        code_name: "Transpose",
        class: ParameterClass::Siren,
        label: "Transpose",
        strip_label: "Transpose",
        unit: "semitones",
        min: -24.0,
        max: 24.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::None,
        widget: Widget::Spin,
        section: Some((0, 2)),
        required_option: None,
    },
    Parameter {
        code_name: "AllSoundOff",
        class: ParameterClass::Siren,
        label: "All Sound Off",
        strip_label: "",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(120),
        widget: Widget::Button,
        section: None,
        required_option: None,
    },
    Parameter {
        code_name: "ResetAllController",
        class: ParameterClass::Siren,
        label: "Reset All Controller",
        strip_label: "",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(121),
        widget: Widget::Button,
        section: None,
        required_option: None,
    },
    Parameter {
        code_name: "AllNoteOff",
        class: ParameterClass::Siren,
        label: "All Note Off",
        strip_label: "",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(123),
        widget: Widget::Button,
        section: None,
        required_option: None,
    },
    Parameter {
        code_name: "Volume",
        class: ParameterClass::Siren,
        label: "Volume",
        strip_label: "Volume",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 127.0,
        midi: Midi::Cc(7),
        widget: Widget::Knob,
        section: Some((4, 2)),
        required_option: None,
    },
    Parameter {
        code_name: "PitchBend",
        class: ParameterClass::Siren,
        label: "Pitch Bend",
        strip_label: "Bend",
        unit: "",
        min: -1.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::PitchWheel,
        widget: Widget::CentredKnob,
        section: Some((0, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "PitchBendRange",
        class: ParameterClass::Siren,
        label: "Pitch Bend Range",
        strip_label: "Bend Range",
        unit: "semitones",
        min: 1.0,
        max: 36.0,
        step: 1.0,
        default: 1.0,
        midi: Midi::Cc(16),
        widget: Widget::Spin,
        section: Some((0, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "VibratoFrequency",
        class: ParameterClass::Siren,
        label: "Vibrato Frequency",
        strip_label: "Speed",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(9),
        widget: Widget::Knob,
        section: Some((1, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "VibratoAmplitude",
        class: ParameterClass::Siren,
        label: "Vibrato Amplitude",
        strip_label: "Depth",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(1),
        widget: Widget::Knob,
        section: Some((1, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "VibratoAcceleration",
        class: ParameterClass::Siren,
        label: "Vibrato Acceleration",
        strip_label: "Evolve",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(11),
        widget: Widget::Knob,
        section: Some((1, 2)),
        required_option: None,
    },
    Parameter {
        code_name: "Portamento",
        class: ParameterClass::Siren,
        label: "Portamento",
        strip_label: "Porta",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(5),
        widget: Widget::Knob,
        section: Some((0, 3)),
        required_option: None,
    },
    Parameter {
        code_name: "TremoloFrequency",
        class: ParameterClass::Siren,
        label: "Tremolo Frequency",
        strip_label: "Speed",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(15),
        widget: Widget::Knob,
        section: Some((2, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "TremoloAmplitude",
        class: ParameterClass::Siren,
        label: "Tremolo Amplitude",
        strip_label: "Depth",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(92),
        widget: Widget::Knob,
        section: Some((2, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "AttackDuration",
        class: ParameterClass::Siren,
        label: "Attack Duration",
        strip_label: "Attack",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(73),
        widget: Widget::Knob,
        section: Some((3, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "ReleaseDuration",
        class: ParameterClass::Siren,
        label: "Release Duration",
        strip_label: "Release",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(72),
        widget: Widget::Knob,
        section: Some((3, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "Timbre",
        class: ParameterClass::Siren,
        label: "Timbre",
        strip_label: "Timbre",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(13),
        widget: Widget::Knob,
        section: Some((4, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "Mute",
        class: ParameterClass::Siren,
        label: "Mute",
        strip_label: "Mute",
        unit: "",
        min: 0.0,
        max: 127.0,
        step: 1.0,
        default: 0.0,
        midi: Midi::Cc(12),
        widget: Widget::Knob,
        section: Some((4, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbEnable",
        class: ParameterClass::Reverb,
        label: "Enable Reverb",
        strip_label: "Enable Reverb",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::Cc(64),
        widget: Widget::Toggle,
        section: Some((5, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbDryWet",
        class: ParameterClass::Reverb,
        label: "Reverb Dry/Wet",
        strip_label: "DryWet",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::Cc(66),
        widget: Widget::Knob,
        section: Some((6, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbDamping",
        class: ParameterClass::Reverb,
        label: "Reverb Damping",
        strip_label: "Damp",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::Cc(67),
        widget: Widget::Knob,
        section: Some((6, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbRoomSize",
        class: ParameterClass::Reverb,
        label: "Reverb Room Size",
        strip_label: "Size",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::Cc(65),
        widget: Widget::Knob,
        section: Some((6, 2)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbWidth",
        class: ParameterClass::Reverb,
        label: "Reverb Width",
        strip_label: "Width",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::Cc(70),
        widget: Widget::Knob,
        section: Some((6, 3)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbLowCut",
        class: ParameterClass::Reverb,
        label: "Reverb Low Cut",
        strip_label: "LowCut",
        unit: "Hz",
        min: 20.0,
        max: 1000.0,
        step: 0.0,
        default: 20.0,
        midi: Midi::Cc(68),
        widget: Widget::Knob,
        section: Some((7, 0)),
        required_option: None,
    },
    Parameter {
        code_name: "ReverbHighCut",
        class: ParameterClass::Reverb,
        label: "Reverb High Cut",
        strip_label: "HighCut",
        unit: "Hz",
        min: 5000.0,
        max: 20000.0,
        step: 0.0,
        default: 20000.0,
        midi: Midi::Cc(69),
        widget: Widget::Knob,
        section: Some((7, 1)),
        required_option: None,
    },
    Parameter {
        code_name: "TrackPanning",
        class: ParameterClass::Track,
        label: "Track Panning",
        strip_label: "",
        unit: "",
        min: -1.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::Cc(10),
        widget: Widget::CentredKnob,
        section: None,
        required_option: None,
    },
    Parameter {
        code_name: "TrackOutputGain",
        class: ParameterClass::Track,
        label: "Track Output Gain",
        strip_label: "",
        unit: "dB",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 1.0,
        midi: Midi::Cc(70),
        widget: Widget::Knob,
        section: None,
        required_option: None,
    },
    Parameter {
        code_name: "MasterVolume",
        class: ParameterClass::Master,
        label: "Master Volume",
        strip_label: "",
        unit: "dB",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 1.0,
        midi: Midi::Cc(7),
        widget: Widget::Knob,
        section: None,
        required_option: None,
    },
    Parameter {
        code_name: "ClicEnable",
        class: ParameterClass::Clic,
        label: "Enable Clic",
        strip_label: "",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 1.0,
        default: 1.0,
        midi: Midi::None,
        widget: Widget::Toggle,
        section: None,
        required_option: Some("COMPOSESIREN_CLIC"),
    },
    Parameter {
        code_name: "ClicVolume",
        class: ParameterClass::Clic,
        label: "Clic Volume",
        strip_label: "",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 1.0,
        midi: Midi::None,
        widget: Widget::Knob,
        section: None,
        required_option: Some("COMPOSESIREN_CLIC"),
    },
    Parameter {
        code_name: "ClicSpread",
        class: ParameterClass::Clic,
        label: "Clic Spread",
        strip_label: "",
        unit: "",
        min: -1.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::None,
        widget: Widget::CentredKnob,
        section: None,
        required_option: Some("COMPOSESIREN_CLIC"),
    },
    Parameter {
        code_name: "ClicBias",
        class: ParameterClass::Clic,
        label: "Clic Bias",
        strip_label: "",
        unit: "",
        min: -1.0,
        max: 1.0,
        step: 0.0,
        default: 0.0,
        midi: Midi::None,
        widget: Widget::CentredKnob,
        section: None,
        required_option: Some("COMPOSESIREN_CLIC"),
    },
    Parameter {
        code_name: "ClicDecay",
        class: ParameterClass::Clic,
        label: "Clic Decay",
        strip_label: "",
        unit: "",
        min: 0.0,
        max: 1.0,
        step: 0.0,
        default: 1.0,
        midi: Midi::None,
        widget: Widget::Knob,
        section: None,
        required_option: Some("COMPOSESIREN_CLIC"),
    },
];

/// The sections, in display order.
pub const SECTIONS: [Section; 8] = [
    Section {
        id: "pitch",
        class: ParameterClass::Siren,
        title: "Pitch",
    },
    Section {
        id: "vibrato",
        class: ParameterClass::Siren,
        title: "Vibrato",
    },
    Section {
        id: "tremolo",
        class: ParameterClass::Siren,
        title: "Tremolo",
    },
    Section {
        id: "envelope",
        class: ParameterClass::Siren,
        title: "Envelope",
    },
    Section {
        id: "output",
        class: ParameterClass::Siren,
        title: "",
    },
    Section {
        id: "reverb_enable",
        class: ParameterClass::Reverb,
        title: "Enable",
    },
    Section {
        id: "reverb",
        class: ParameterClass::Reverb,
        title: "Reverb",
    },
    Section {
        id: "reverb_filter",
        class: ParameterClass::Reverb,
        title: "Filter",
    },
];

/// The sirens.
pub const SIRENS: [SirenUi; 7] = [
    SirenUi {
        id: "S1",
        strip_colour: 0xff_29_5d_d1,
        category_default: true,
    },
    SirenUi {
        id: "S2",
        strip_colour: 0xff_1f_6c_d1,
        category_default: false,
    },
    SirenUi {
        id: "S3",
        strip_colour: 0xff_46_50_c8,
        category_default: true,
    },
    SirenUi {
        id: "S4",
        strip_colour: 0xff_36_54_ce,
        category_default: true,
    },
    SirenUi {
        id: "S5",
        strip_colour: 0xff_17_7c_cf,
        category_default: true,
    },
    SirenUi {
        id: "S6",
        strip_colour: 0xff_0e_8e_ce,
        category_default: false,
    },
    SirenUi {
        id: "S7",
        strip_colour: 0xff_07_a0_cb,
        category_default: true,
    },
];

/// Colours (0xAARRGGBB) and lengths (logical pixels).
pub mod theme {
    /// Accent: knob arcs, the held keyboard key, selections.
    pub const COLOUR_ORANGE_MECANIQUE: u32 = 0xff_ff_99_00;
    /// Background of the overlay panels.
    pub const COLOUR_DARK_TRANSPARENT_BACKGROUND: u32 = 0xf2_28_35_41;
    /// Background of a siren strip.
    pub const COLOUR_BACKGROUND_STRIP_GREY: u32 = 0xff_31_41_59;
    /// First colour of the strip ramp (the lowest siren).
    pub const COLOUR_SIREN_RAMP_DARK_BLUE: u32 = 0xff_46_50_c8;
    /// Last colour of the strip ramp.
    pub const COLOUR_SIREN_RAMP_LIGHT_BLUE: u32 = 0xff_00_b4_c8;
    /// Siren palette.
    pub const COLOUR_SIREN_RAMP_DARK_GREEN: u32 = 0xff_3c_8c_28;
    /// Siren palette.
    pub const COLOUR_SIREN_RAMP_LIGHT_GREEN: u32 = 0xff_78_b4_28;
    /// Siren palette.
    pub const COLOUR_SIREN_RAMP_SUNNY_YELLOW: u32 = 0xff_d7_b7_00;
    /// Siren palette.
    pub const COLOUR_SIREN_RAMP_LIGHT_ORANGE: u32 = 0xff_ff_7f_00;
    /// Siren palette.
    pub const COLOUR_SIREN_RAMP_DARK_ORANGE: u32 = 0xff_ff_45_00;
    /// Keyboard level meter: low.
    pub const COLOUR_MIDI_KEYBOARD_LOW_LEVEL_RED: u32 = 0xff_80_00_00;
    /// Keyboard level meter: middle.
    pub const COLOUR_MIDI_KEYBOARD_MID_LEVEL_RED: u32 = 0xff_ff_00_00;
    /// Keyboard level meter: high.
    pub const COLOUR_MIDI_KEYBOARD_HIGH_LEVEL_RED: u32 = 0xff_ff_66_00;
    /// Keyboard level meter: the dB range separators.
    pub const COLOUR_MIDI_KEYBOARD_DB_RANGE_SEPARATOR_BLUE: u32 = 0xff_00_00_ff;
    /// Width of the siren title cell at the left of a strip.
    pub const STRIP_TITLE_AREA_WIDTH: f32 = 70.0;
    /// Font size of the siren title.
    pub const STRIP_TITLE_FONT_SIZE: f32 = 13.0;
    /// Corner radius of a strip.
    pub const STRIP_CORNER_SIZE: f32 = 10.0;
    /// Gap between the cells of a strip.
    pub const STRIP_SPACER_SIZE: f32 = 2.0;
    /// Height of a section title.
    pub const STRIP_GROUP_LABEL_HEIGHT: f32 = 16.0;
    /// Font size of a section title.
    pub const STRIP_GROUP_LABEL_FONT_SIZE: f32 = 12.0;
    /// Height of a knob caption (two lines: the label, then the CC number).
    pub const STRIP_SLIDER_LABEL_HEIGHT: f32 = 28.0;
    /// Font size of a knob caption.
    pub const STRIP_SLIDER_LABEL_FONT_SIZE: f32 = 11.5;
    /// Smallest height of a knob.
    pub const STRIP_MIN_SLIDER_HEIGHT: f32 = 62.0;
    /// Smallest height of a whole strip.
    pub const STRIP_MIN_FULL_STRIP_HEIGHT: f32 = 100.0;
    /// Smallest width of a knob cell.
    pub const STRIP_MIN_KNOB_SLIDER_WIDTH: f32 = 47.0;
    /// Smallest width of an up/down value cell.
    pub const STRIP_MIN_INC_DEC_SLIDER_WIDTH: f32 = 70.0;
    /// Width of the knob track.
    pub const STRIP_KNOB_INDICATOR_OFF_THICKNESS: f32 = 2.0;
    /// Width of the knob value arc.
    pub const STRIP_KNOB_INDICATOR_ON_THICKNESS: f32 = 4.0;
    /// Initial width of the OneSiren editor.
    pub const EDITOR_ONE_SIREN_WIDTH: f32 = 754.0;
    /// Initial height of the OneSiren editor.
    pub const EDITOR_ONE_SIREN_HEIGHT: f32 = 200.0;
}
