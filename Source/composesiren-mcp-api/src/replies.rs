//! The replies clients read (the server sends more fields; these are the ones used).

use serde::Deserialize;

/// `list_midi_devices`.
#[derive(Debug, Clone, Deserialize)]
pub struct MidiDevices {
    /// The MIDI inputs the standalone can open.
    #[serde(default)]
    pub inputs: Vec<MidiInput>,
    /// The MIDI outputs.
    #[serde(default)]
    pub outputs: Vec<MidiOutput>,
}

/// One MIDI input.
#[derive(Debug, Clone, Deserialize)]
pub struct MidiInput {
    /// The device's name, as the system lists it.
    pub name: String,
    /// What `set_midi_input` takes.
    pub identifier: String,
    /// Whether the standalone listens to it.
    #[serde(default)]
    pub enabled: bool,
}

/// One MIDI output.
#[derive(Debug, Clone, Deserialize)]
pub struct MidiOutput {
    /// The device's name, as the system lists it.
    pub name: String,
    /// What `set_midi_output` takes.
    pub identifier: String,
    /// Whether it is the default output.
    #[serde(default)]
    pub selected: bool,
}

#[cfg(test)]
mod tests {
    use super::MidiDevices;

    #[test]
    fn reads_what_the_standalone_sends() {
        let reply = r#"{"ok":true,"inputs":[{"name":"tap-viewer","identifier":"A1","enabled":false}],
            "outputs":[{"name":"Out","identifier":"B2","selected":true}]}"#;
        let devices: MidiDevices = serde_json::from_str(reply).unwrap();
        assert_eq!(devices.inputs[0].identifier, "A1");
        assert!(!devices.inputs[0].enabled);
        assert!(devices.outputs[0].selected);
    }
}
