//! The discovery file.

use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

/// One running server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// Plugin name, for example "OneSiren" or "SirenOrchestra".
    pub plugin_name: String,
    /// Four-character plugin code, for example "MvOS". Gearmulator calls this `plugin4CC`.
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

/// An application's discovery file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    path: PathBuf,
}

impl Registry {
    /// The file at `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Registry { path: path.into() }
    }

    /// `file_name` in the home folder (`$HOME`, or `%USERPROFILE%`): `.composesiren_mcp.json`.
    #[must_use]
    pub fn in_home(file_name: &str) -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        Registry::new(home.join(file_name))
    }

    /// Where the file is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The entries of the file: none if it is missing or empty.
    pub fn read(&self) -> Result<Vec<Instance>, String> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("{}: {e}", self.path.display())),
        };
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", self.path.display()))
    }

    /// The entries whose process still exists.
    pub fn running(&self) -> Result<Vec<Instance>, String> {
        Ok(self.read()?.into_iter().filter(|i| pid_alive(i.pid)).collect())
    }

    /// Insert `instance`, dropping any previous entry on the same port and any entry whose
    /// process is gone.
    pub fn register(&self, instance: &Instance) -> io::Result<()> {
        self.edit(|instances| {
            instances.retain(|entry| entry.port != instance.port && pid_alive(entry.pid));
            instances.push(instance.clone());
        })
    }

    /// Remove the entry for `port` (and any whose process is gone).
    pub fn unregister(&self, port: u16) -> io::Result<()> {
        self.edit(|instances| {
            instances.retain(|entry| entry.port != port && pid_alive(entry.pid));
        })
    }

    /// Reads, edits and writes the file under an exclusive lock.
    fn edit(&self, change: impl FnOnce(&mut Vec<Instance>)) -> io::Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut file =
            OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&self.path)?;
        file.lock_exclusive()?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let mut instances = if text.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str::<Vec<Instance>>(&text).unwrap_or_default()
        };
        change(&mut instances);
        let body = serde_json::to_string_pretty(&instances)?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(body.as_bytes())?;
        file.write_all(b"\n")?;
        file.unlock()?;
        Ok(())
    }
}

/// Session id inherited from the environment (`CLAUDE_CODE_SESSION_ID`), or empty.
#[must_use]
pub fn session_id_from_env() -> String {
    std::env::var("CLAUDE_CODE_SESSION_ID").unwrap_or_default()
}

/// Current process id.
#[must_use]
pub fn current_pid() -> u32 {
    std::process::id()
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
        io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file in a fresh folder of its own.
    fn registry(name: &str) -> (Registry, PathBuf) {
        let dir = std::env::temp_dir().join(format!("mcp-discovery-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (Registry::new(dir.join(".x_mcp.json")), dir)
    }

    fn sample(port: u16, pid: u32) -> Instance {
        Instance {
            plugin_name: "OneSiren".to_owned(),
            plugin_4cc: "MvOS".to_owned(),
            port,
            pid,
            session_id: String::new(),
            standalone: true,
        }
    }

    #[test]
    fn register_then_unregister_round_trips() {
        let (registry, dir) = registry("round-trip");
        registry.register(&sample(13720, current_pid())).unwrap();
        registry.register(&sample(13721, current_pid())).unwrap();
        let read = registry.read().unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].plugin_4cc, "MvOS");
        assert!(read[1].standalone);
        registry.unregister(13720).unwrap();
        assert_eq!(registry.read().unwrap().len(), 1);
        assert_eq!(registry.read().unwrap()[0].port, 13721);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn registering_a_port_again_replaces_its_entry() {
        let (registry, dir) = registry("replace");
        registry.register(&sample(13720, current_pid())).unwrap();
        let mut again = sample(13720, current_pid());
        again.plugin_name = "SirenOrchestra".to_owned();
        registry.register(&again).unwrap();
        let read = registry.read().unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].plugin_name, "SirenOrchestra");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_entry_whose_process_is_gone_is_not_running_and_is_dropped_on_register() {
        let (registry, dir) = registry("stale");
        // A pid that does not exist.
        std::fs::write(
            registry.path(),
            r#"[{"pluginName":"Old","plugin4CC":"MvOS","port":13720,"pid":999999,"sessionId":"","standalone":true}]"#,
        )
        .unwrap();
        assert_eq!(registry.read().unwrap().len(), 1);
        assert!(registry.running().unwrap().is_empty());
        registry.register(&sample(13721, current_pid())).unwrap();
        assert_eq!(registry.read().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// The file as Gearmulator writes it, with a field this version does not know.
    #[test]
    fn reads_the_shape_gearmulator_uses_and_ignores_unknown_fields() {
        let (registry, dir) = registry("shape");
        std::fs::write(
            registry.path(),
            r#"[{"pluginName":"SirenOrchestra","plugin4CC":"MvSO","port":13720,"pid":1,
                 "sessionId":"abc","standalone":false,"startedAt":"2026-10-02"}]"#,
        )
        .unwrap();
        let read = registry.read().unwrap();
        assert_eq!(read[0].plugin_4cc, "MvSO");
        assert_eq!(read[0].session_id, "abc");
        assert!(!read[0].standalone);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_or_empty_file_is_no_instance() {
        let (registry, dir) = registry("missing");
        assert!(registry.read().unwrap().is_empty());
        std::fs::write(registry.path(), "  \n").unwrap();
        assert!(registry.read().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn this_process_is_alive_and_pid_zero_is_not() {
        assert!(pid_alive(current_pid()));
        assert!(!pid_alive(0));
    }
}
