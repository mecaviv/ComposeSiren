//! MCP tools. Each one forwards a JSON command to the plugin.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerConfig};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::dispatch::Dispatch;

/// The MCP endpoint mounted at `/mcp`.
#[derive(Clone)]
pub struct ComposeSirenServer {
    dispatch: Dispatch,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ParameterIdArgs {
    /// JUCE parameter id, for example `S1 | Volume` or `S | PitchBend`.
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetParameterArgs {
    /// JUCE parameter id, for example `S1 | Volume`.
    id: String,
    /// Value in the parameter's own units, not the normalised 0..1 range.
    value: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SendMidiArgs {
    /// MIDI status byte, 0 to 255.
    status: u8,
    /// First data byte, 0 to 127.
    data1: u8,
    /// Second data byte, 0 to 127.
    #[serde(default)]
    data2: u8,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SendNoteArgs {
    /// MIDI channel, 1 to 16. SirenOrchestra uses the channel to pick the siren.
    channel: u8,
    /// Note number, 0 to 127.
    note: u8,
    /// Velocity, 0 to 127. Zero sends a note off.
    velocity: u8,
    /// Milliseconds before the matching note off. Zero sends no note off.
    #[serde(default)]
    duration_ms: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetAudioDeviceArgs {
    /// Output device name. Omit to keep the current output.
    #[serde(default)]
    output: Option<String>,
    /// Input device name. An empty string selects no input. Omit to keep the current input.
    #[serde(default)]
    input: Option<String>,
    /// Sample rate in Hz. Omit to keep the current rate.
    #[serde(default)]
    sample_rate: Option<f64>,
    /// Buffer size in samples. Omit to keep the current size.
    #[serde(default)]
    buffer_size: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetMidiInputArgs {
    /// Device identifier from `list_midi_devices`.
    identifier: String,
    /// Open or close the device.
    enabled: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetMidiOutputArgs {
    /// Device identifier from `list_midi_devices`. Empty clears the default output.
    identifier: String,
}

#[tool_router]
impl ComposeSirenServer {
    pub fn new(dispatch: Dispatch) -> Self {
        #[cfg(feature = "record")]
        let tool_router = Self::tool_router() + Self::record_router();
        #[cfg(not(feature = "record"))]
        let tool_router = Self::tool_router();
        Self { dispatch, tool_router }
    }

    #[tool(description = "List every automatable parameter with its id, value, and range.")]
    fn list_parameters(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "list_parameters"})))
    }

    #[tool(description = "Read one parameter by its JUCE id.")]
    fn get_parameter(
        &self,
        Parameters(args): Parameters<ParameterIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "get_parameter", "id": args.id})))
    }

    #[tool(description = "Set one parameter by its JUCE id. The value uses the parameter's own units.")]
    fn set_parameter(
        &self,
        Parameters(args): Parameters<SetParameterArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(
            self.dispatch
                .call(json!({"op": "set_parameter", "id": args.id, "value": args.value})),
        )
    }

    #[tool(description = "Queue a MIDI message into the next audio block, as if the host had sent it.")]
    fn send_midi(&self, Parameters(args): Parameters<SendMidiArgs>) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "send_midi",
            "status": args.status,
            "data1": args.data1,
            "data2": args.data2,
        })))
    }

    #[tool(description = "Send a note on, and a note off after duration_ms. Channel 1 is siren S1 in SirenOrchestra.")]
    fn send_note(&self, Parameters(args): Parameters<SendNoteArgs>) -> Result<CallToolResult, McpError> {
        if !(1..=16).contains(&args.channel) {
            return Err(McpError::invalid_params("channel must be 1 to 16", None));
        }
        let status_on = 0x90 | (args.channel - 1);
        let started = self.dispatch.call(json!({
            "op": "send_midi",
            "status": status_on,
            "data1": args.note,
            "data2": args.velocity,
        }));
        if started.get("ok").and_then(Value::as_bool) == Some(false) {
            return self.finish(started);
        }
        let duration = args.duration_ms.min(10_000);
        if duration > 0 && args.velocity > 0 {
            std::thread::sleep(std::time::Duration::from_millis(duration));
            let stopped = self.dispatch.call(json!({
                "op": "send_midi",
                "status": status_on,
                "data1": args.note,
                "data2": 0,
            }));
            return self.finish(stopped);
        }
        self.finish(started)
    }

    #[tool(description = "List audio input and output devices. Only the standalone owns them.")]
    fn list_audio_devices(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "list_audio_devices"})))
    }

    #[tool(description = "Switch the standalone's audio device, sample rate, or buffer size. This restarts the device.")]
    fn set_audio_device(
        &self,
        Parameters(args): Parameters<SetAudioDeviceArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_audio_device",
            "output": args.output,
            "input": args.input,
            "sampleRate": args.sample_rate,
            "bufferSize": args.buffer_size,
        })))
    }

    #[tool(description = "List MIDI inputs and outputs. Only the standalone owns them.")]
    fn list_midi_devices(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "list_midi_devices"})))
    }

    #[tool(description = "Open or close a MIDI input on the standalone.")]
    fn set_midi_input(
        &self,
        Parameters(args): Parameters<SetMidiInputArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_midi_input",
            "identifier": args.identifier,
            "enabled": args.enabled,
        })))
    }

    #[tool(description = "Choose the standalone's default MIDI output. Pass an empty identifier to clear it.")]
    fn set_midi_output(
        &self,
        Parameters(args): Parameters<SetMidiOutputArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_midi_output",
            "identifier": args.identifier,
        })))
    }

    fn finish(&self, value: Value) -> Result<CallToolResult, McpError> {
        if value.get("ok").and_then(Value::as_bool) == Some(false) {
            let message = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("the plugin rejected the command")
                .to_owned();
            return Err(McpError::invalid_params(message, Some(value)));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            value.to_string(),
        )]))
    }
}

#[cfg(feature = "record")]
#[derive(Debug, Deserialize, JsonSchema)]
struct StartRecordingArgs {
    /// File to write. Omit for ~/Music/ComposeSiren/ComposeSiren-<date>-<time>.<ext>.
    #[serde(default)]
    path: Option<String>,
    /// `flac` (24-bit, the default), `wav` (24-bit) or `wav-float` (32-bit float).
    #[serde(default)]
    format: Option<String>,
}

#[cfg(feature = "record")]
#[derive(Debug, Deserialize, JsonSchema)]
struct StopRecordingArgs {
    /// End without cutting a sound short: wait for it to die out, and if it
    /// still sounds (a drone), fade it out. The reply comes at once; call
    /// `recording_status` until `recording` is false.
    #[serde(default)]
    fade: bool,
    /// Seconds to wait for the sound to die out (default 2).
    #[serde(default)]
    wait_seconds: Option<f64>,
    /// Seconds the fade-out lasts (default 3).
    #[serde(default)]
    fade_seconds: Option<f64>,
}

/// The recording tools, with the `record` feature (`COMPOSESIREN_RECORD`).
#[cfg(feature = "record")]
#[tool_router(router = record_router)]
impl ComposeSirenServer {
    #[tool(description = "Start recording the audio output to a FLAC or WAV file. Returns the file's path.")]
    fn start_recording(
        &self,
        Parameters(args): Parameters<StartRecordingArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "start_recording",
            "path": args.path,
            "format": args.format,
        })))
    }

    #[tool(description = "Stop recording and complete the file, at once or (fade) once the sound has died out or faded out. Returns its path, duration and dropped frames.")]
    fn stop_recording(&self, Parameters(args): Parameters<StopRecordingArgs>) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "stop_recording",
            "fade": args.fade,
            "wait_seconds": args.wait_seconds,
            "fade_seconds": args.fade_seconds,
        })))
    }

    #[tool(description = "Whether a recording is running, its file, duration so far and dropped frames.")]
    fn recording_status(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "recording_status"})))
    }
}

// The field, not the macro's default `Self::tool_router()`: with the `record`
// feature, `new` adds the recording tools to it.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for ComposeSirenServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "ComposeSiren remote control. Parameter ids look like \"S1 | Volume\". \
             Audio and MIDI device tools work on the standalone; a plugin hosted in a DAW \
             reports that the host owns the devices. Running instances are listed in \
             ~/.composesiren_mcp.json."
                .to_owned(),
        )
    }
}
