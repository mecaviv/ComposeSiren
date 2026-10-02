//! The arguments each tool takes. The doc comments are the tools' schema
//! descriptions (with the `schema` feature).

use serde::{Deserialize, Serialize};

/// `get_parameter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParameterId {
    /// JUCE parameter id, for example `S1 | Volume` or `S | PitchBend`.
    pub id: String,
}

/// `set_parameter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetParameter {
    /// JUCE parameter id, for example `S1 | Volume`.
    pub id: String,
    /// Value in the parameter's own units, not the normalised 0..1 range.
    pub value: f64,
}

/// `send_midi`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SendMidi {
    /// MIDI status byte, 0 to 255.
    pub status: u8,
    /// First data byte, 0 to 127.
    pub data1: u8,
    /// Second data byte, 0 to 127.
    #[serde(default)]
    pub data2: u8,
}

/// `send_note`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SendNote {
    /// MIDI channel, 1 to 16. SirenOrchestra uses the channel to pick the siren.
    pub channel: u8,
    /// Note number, 0 to 127.
    pub note: u8,
    /// Velocity, 0 to 127. Zero sends a note off.
    pub velocity: u8,
    /// Milliseconds before the matching note off. Zero sends no note off.
    #[serde(default)]
    pub duration_ms: u64,
}

/// `set_audio_device`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetAudioDevice {
    /// Output device name. Omit to keep the current output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Input device name. An empty string selects no input. Omit to keep the current input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// Sample rate in Hz. Omit to keep the current rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<f64>,
    /// Buffer size in samples. Omit to keep the current size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_size: Option<i32>,
}

/// `set_midi_input`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetMidiInput {
    /// Device identifier from `list_midi_devices`.
    pub identifier: String,
    /// Open or close the device.
    pub enabled: bool,
}

/// `set_midi_output`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetMidiOutput {
    /// Device identifier from `list_midi_devices`. Empty clears the default output.
    pub identifier: String,
}

/// `get_setting`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GetSetting {
    /// Setting id from `list_settings`, e.g. `clic.output_device`.
    pub id: String,
}

/// `set_setting`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetSetting {
    /// Setting id from `list_settings`, e.g. `clic.output_device`.
    pub id: String,
    /// String settings take text (a device name or "Main output"); the others take a number.
    pub value: serde_json::Value,
}

/// `set_song_title`: show this song in the UI (title bar) until
/// `clear_song_title`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetSongTitle {
    /// The song's title, usually its file name.
    pub title: String,
    /// How long the song is, in seconds, when known: the UI draws a progress
    /// bar. Omit when it is not known yet (no bar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

/// `set_song_progress`: how far the song is. The title is already set with
/// `set_song_title`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SetSongProgress {
    /// How many seconds of the song have played.
    pub position_seconds: f64,
    /// How long the song is, in seconds, when it has become known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

/// `start_recording`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StartRecording {
    /// File to write. Omit for ~/Music/ComposeSiren/ComposeSiren-<date>-<time>.<ext>.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `flac` (24-bit, the default), `wav` (24-bit) or `wav-float` (32-bit float).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// `stop_recording`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StopRecording {
    /// End without cutting a sound short: wait for it to die out, and if it
    /// still sounds (a drone), fade it out. The reply comes at once; call
    /// `recording_status` until `recording` is false.
    #[serde(default)]
    pub fade: bool,
    /// Seconds to wait for the sound to die out (default 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_seconds: Option<f64>,
    /// Seconds the fade-out lasts (default 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_seconds: Option<f64>,
}
