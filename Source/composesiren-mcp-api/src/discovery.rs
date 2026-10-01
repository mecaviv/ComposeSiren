//! `~/.composesiren_mcp.json`: each running instance lists itself there when
//! its server binds a port, in the shape Gearmulator uses for
//! `~/.gearmulator_mcp.json`. Clients read it to learn which port to open.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One running ComposeSiren instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// Plugin name, "OneSiren" or "SirenOrchestra".
    pub plugin_name: String,
    /// Four-character plugin code, "MvOS" or "MvSO". Gearmulator calls this `plugin4CC`.
    #[serde(rename = "plugin4CC")]
    pub plugin_4cc: String,
    /// TCP port of the MCP HTTP server on 127.0.0.1.
    pub port: u16,
    /// Operating-system process id of the host.
    pub pid: u32,
    /// `CLAUDE_CODE_SESSION_ID` of the host process, or empty.
    pub session_id: String,
    /// True when this process is the standalone and can change audio and MIDI devices.
    pub standalone: bool,
}

/// `~/.composesiren_mcp.json`, or `%USERPROFILE%\.composesiren_mcp.json`.
#[must_use]
pub fn default_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    home.join(".composesiren_mcp.json")
}

/// The entries of the discovery file at `path`: none if it is missing or empty.
pub fn read(path: &std::path::Path) -> Result<Vec<Instance>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The instances whose process still exists, in the default discovery file.
pub fn running() -> Result<Vec<Instance>, String> {
    Ok(read(&default_path())?.into_iter().filter(|i| pid_alive(i.pid)).collect())
}

/// Whether a process with this id exists.
#[must_use]
#[allow(unsafe_code)]
pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let Ok(raw) = i32::try_from(pid) else { return false };
        // SAFETY: `kill` with signal 0 only checks that the process exists.
        let rc = unsafe { libc::kill(raw, 0) };
        if rc == 0 {
            return true;
        }
        // EPERM: the process exists but belongs to another user.
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Instance, pid_alive, read};

    #[test]
    fn reads_the_file_the_server_writes() {
        let text = r#"[{"pluginName":"SirenOrchestra","plugin4CC":"MvSO","port":13720,"pid":1,"sessionId":"","standalone":true}]"#;
        let instances: Vec<Instance> = serde_json::from_str(text).unwrap();
        assert_eq!(instances[0].plugin_4cc, "MvSO");
        assert!(instances[0].standalone);
    }

    #[test]
    fn a_missing_file_is_no_instance() {
        assert!(read(std::path::Path::new("/nonexistent/.composesiren_mcp.json")).unwrap().is_empty());
    }

    #[test]
    fn this_process_is_alive_and_pid_zero_is_not() {
        assert!(pid_alive(std::process::id()));
        assert!(!pid_alive(0));
    }
}
