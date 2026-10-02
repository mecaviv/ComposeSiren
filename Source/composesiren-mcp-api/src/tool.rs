//! The tools' names.

/// `list_parameters`.
pub const LIST_PARAMETERS: &str = "list_parameters";
/// `get_parameter`.
pub const GET_PARAMETER: &str = "get_parameter";
/// `set_parameter`.
pub const SET_PARAMETER: &str = "set_parameter";
/// `send_midi`.
pub const SEND_MIDI: &str = "send_midi";
/// `send_note`.
pub const SEND_NOTE: &str = "send_note";
/// `list_audio_devices`.
pub const LIST_AUDIO_DEVICES: &str = "list_audio_devices";
/// `set_audio_device`.
pub const SET_AUDIO_DEVICE: &str = "set_audio_device";
/// `list_midi_devices`.
pub const LIST_MIDI_DEVICES: &str = "list_midi_devices";
/// `set_midi_input`.
pub const SET_MIDI_INPUT: &str = "set_midi_input";
/// `set_midi_output`.
pub const SET_MIDI_OUTPUT: &str = "set_midi_output";
/// `list_settings`.
pub const LIST_SETTINGS: &str = "list_settings";
/// `get_setting`.
pub const GET_SETTING: &str = "get_setting";
/// `set_setting`.
pub const SET_SETTING: &str = "set_setting";
/// `reset_controllers`.
pub const RESET_CONTROLLERS: &str = "reset_controllers";
/// `set_song_title` (a build with `COMPOSESIREN_SONG_TITLE`): show the playing
/// song in the UI (title bar) with an optional progress bar. `tap-viewer midi
/// --song` calls it and goes on without it when the tool is missing.
pub const SET_SONG_TITLE: &str = "set_song_title";
/// `set_song_progress` (a build with `COMPOSESIREN_SONG_TITLE`): how far the
/// song is (the title is already set).
pub const SET_SONG_PROGRESS: &str = "set_song_progress";
/// `clear_song_title` (a build with `COMPOSESIREN_SONG_TITLE`): the song is
/// over; the title bar goes back to normal.
pub const CLEAR_SONG_TITLE: &str = "clear_song_title";
/// `start_recording` (a build with `COMPOSESIREN_RECORD`).
pub const START_RECORDING: &str = "start_recording";
/// `stop_recording` (a build with `COMPOSESIREN_RECORD`).
pub const STOP_RECORDING: &str = "stop_recording";
/// `recording_status` (a build with `COMPOSESIREN_RECORD`).
pub const RECORDING_STATUS: &str = "recording_status";

/// Every server has these.
pub const ALWAYS: &[&str] = &[
    LIST_PARAMETERS,
    GET_PARAMETER,
    SET_PARAMETER,
    SEND_MIDI,
    SEND_NOTE,
    LIST_AUDIO_DEVICES,
    SET_AUDIO_DEVICE,
    LIST_MIDI_DEVICES,
    SET_MIDI_INPUT,
    SET_MIDI_OUTPUT,
    LIST_SETTINGS,
    GET_SETTING,
    SET_SETTING,
    RESET_CONTROLLERS,
];

/// Only a build with the song title bar has these.
pub const SONG_TITLE: &[&str] = &[SET_SONG_TITLE, SET_SONG_PROGRESS, CLEAR_SONG_TITLE];

/// Only a build with the recorder has these.
pub const RECORDING: &[&str] = &[START_RECORDING, STOP_RECORDING, RECORDING_STATUS];
