//! MCP tools. Each one forwards a JSON command to the plugin.

use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler};
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerConfig};
use serde_json::{Value, json};

use std::sync::Arc;

use composesiren_mcp_api::args::{
    GetSetting, ParameterId, ResetControllers, SendMidi, SendNote, SetAudioDevice, SetMidiInput, SetMidiOutput,
    SetParameter, SetSetting,
};
#[cfg(feature = "record")]
use composesiren_mcp_api::args::{StartRecording, StopRecording};
#[cfg(feature = "song-title")]
use composesiren_mcp_api::args::{SetSongProgress, SetSongTitle};

use crate::dispatch::Dispatch;
use crate::stats::Stats;

/// The MCP endpoint mounted at `/mcp`.
#[derive(Clone)]
pub struct ComposeSirenServer {
    dispatch: Dispatch,
    stats: Arc<Stats>,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[tool_router]
impl ComposeSirenServer {
    pub fn new(dispatch: Dispatch, stats: Arc<Stats>) -> Self {
        let tool_router = Self::tool_router();
        #[cfg(feature = "record")]
        let tool_router = tool_router + Self::record_router();
        #[cfg(feature = "song-title")]
        let tool_router = tool_router + Self::song_title_router();
        Self { dispatch, stats, tool_router }
    }

    #[tool(description = "List every automatable parameter with its id, value, and range.")]
    fn list_parameters(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "list_parameters"})))
    }

    #[tool(description = "Read one parameter by its JUCE id.")]
    fn get_parameter(
        &self,
        Parameters(args): Parameters<ParameterId>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "get_parameter", "id": args.id})))
    }

    #[tool(description = "Set one parameter by its JUCE id. The value uses the parameter's own units.")]
    fn set_parameter(
        &self,
        Parameters(args): Parameters<SetParameter>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(
            self.dispatch
                .call(json!({"op": "set_parameter", "id": args.id, "value": args.value})),
        )
    }

    #[tool(description = "Queue a MIDI message into the next audio block, as if the host had sent it.")]
    fn send_midi(&self, Parameters(args): Parameters<SendMidi>) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "send_midi",
            "status": args.status,
            "data1": args.data1,
            "data2": args.data2,
        })))
    }

    #[tool(description = "Send a note on, and a note off after duration_ms. Channel 1 is siren S1 in SirenOrchestra.")]
    fn send_note(&self, Parameters(args): Parameters<SendNote>) -> Result<CallToolResult, McpError> {
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

    #[tool(description = "Reset the controllers of one siren, or of every siren when `siren` is omitted, like MIDI CC 121. \
        Volume, pitch bend, pitch bend range, vibrato, tremolo, portamento, attack and release, transpose and the other \
        siren parameters go back to their defaults, and sounding notes are cut. Reverb and master settings are kept.")]
    fn reset_controllers(
        &self,
        Parameters(args): Parameters<ResetControllers>,
    ) -> Result<CallToolResult, McpError> {
        if args.siren.is_some_and(|siren| !(1..=7).contains(&siren)) {
            return Err(McpError::invalid_params("siren must be 1 to 7", None));
        }
        self.finish(self.dispatch.call(json!({
            "op": "reset_controllers",
            "siren": args.siren.unwrap_or(0),
        })))
    }

    #[tool(description = "List audio input and output devices. Only the standalone owns them.")]
    fn list_audio_devices(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "list_audio_devices"})))
    }

    #[tool(description = "Switch the standalone's audio device, sample rate, or buffer size. This restarts the device.")]
    fn set_audio_device(
        &self,
        Parameters(args): Parameters<SetAudioDevice>,
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
        Parameters(args): Parameters<SetMidiInput>,
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
        Parameters(args): Parameters<SetMidiOutput>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_midi_output",
            "identifier": args.identifier,
        })))
    }

    #[tool(description = "List power-user settings (metadata id, type, value).")]
    fn list_settings(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "list_settings"})))
    }

    #[tool(description = "Read one power-user setting by its metadata id (e.g. clic.output_device).")]
    fn get_setting(
        &self,
        Parameters(args): Parameters<GetSetting>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "get_setting", "id": args.id})))
    }

    #[tool(description = "Set one power-user setting. String settings take a device name or text; the others take a number.")]
    fn set_setting(
        &self,
        Parameters(args): Parameters<SetSetting>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_setting",
            "id": args.id,
            "value": args.value,
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

/// The song title tools, with the `song-title` feature (`COMPOSESIREN_SONG_TITLE`).
#[cfg(feature = "song-title")]
#[tool_router(router = song_title_router)]
impl ComposeSirenServer {
    #[tool(description = "Show the playing song in the UI title bar, with an optional progress bar when its length is known. Call clear_song_title when it ends.")]
    fn set_song_title(
        &self,
        Parameters(args): Parameters<SetSongTitle>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_song_title",
            "title": args.title,
            "duration_seconds": args.duration_seconds,
        })))
    }

    #[tool(description = "How far the song shown by set_song_title has played, so the progress bar can keep up with the board.")]
    fn set_song_progress(
        &self,
        Parameters(args): Parameters<SetSongProgress>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "set_song_progress",
            "position_seconds": args.position_seconds,
            "duration_seconds": args.duration_seconds,
        })))
    }

    #[tool(description = "The song is over: the title bar goes back to its usual look.")]
    fn clear_song_title(&self) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({"op": "clear_song_title"})))
    }
}

/// The recording tools, with the `record` feature (`COMPOSESIREN_RECORD`).
#[cfg(feature = "record")]
#[tool_router(router = record_router)]
impl ComposeSirenServer {
    #[tool(description = "Start recording the audio output to a FLAC or WAV file. Returns the file's path.")]
    fn start_recording(
        &self,
        Parameters(args): Parameters<StartRecording>,
    ) -> Result<CallToolResult, McpError> {
        self.finish(self.dispatch.call(json!({
            "op": "start_recording",
            "path": args.path,
            "format": args.format,
        })))
    }

    #[tool(description = "Stop recording and complete the file, at once or (fade) once the sound has died out or faded out. Returns its path, duration and dropped frames.")]
    fn stop_recording(&self, Parameters(args): Parameters<StopRecording>) -> Result<CallToolResult, McpError> {
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
// or `song-title` features, `new` adds those tools to it.
//
// `call_tool` is written out, rather than left to the macro, to count each tool.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for ComposeSirenServer {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let name = request.name.to_string();
        let result = self
            .tool_router
            .call(ToolCallContext::new(self, request, context))
            .await;
        self.stats.record(&name, result.is_err());
        result
    }

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

#[cfg(test)]
mod tests {
    use composesiren_mcp_api::tool;

    use super::ComposeSirenServer;

    /// The tools the server offers are the ones `composesiren-mcp-api` declares, so a
    /// client built on it never asks for one that is not there, or misses a new one.
    #[test]
    fn the_tools_are_the_ones_the_api_declares() {
        let mut offered: Vec<String> =
            ComposeSirenServer::tool_router().list_all().iter().map(|t| t.name.to_string()).collect();
        offered.sort();
        let mut declared: Vec<String> = tool::ALWAYS.iter().map(|t| (*t).to_owned()).collect();
        declared.sort();
        assert_eq!(offered, declared);
    }

    #[cfg(feature = "song-title")]
    #[test]
    fn the_song_title_tools_are_the_ones_the_api_declares() {
        let router = ComposeSirenServer::song_title_router();
        let mut offered: Vec<String> = router.list_all().iter().map(|t| t.name.to_string()).collect();
        offered.sort();
        let mut declared: Vec<String> = tool::SONG_TITLE.iter().map(|t| (*t).to_owned()).collect();
        declared.sort();
        assert_eq!(offered, declared);
    }

    #[cfg(feature = "record")]
    #[test]
    fn the_recording_tools_are_the_ones_the_api_declares() {
        let router = ComposeSirenServer::record_router();
        let mut offered: Vec<String> = router.list_all().iter().map(|t| t.name.to_string()).collect();
        offered.sort();
        let mut declared: Vec<String> = tool::RECORDING.iter().map(|t| (*t).to_owned()).collect();
        declared.sort();
        assert_eq!(offered, declared);
    }
}
