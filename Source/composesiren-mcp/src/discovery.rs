//! Discovery file, in the shape Gearmulator uses for `~/.gearmulator_mcp.json`.
//!
//! Each running plugin or standalone appends itself when its server binds a
//! port, and removes itself when the server stops. Clients read the file to
//! learn which port to open. `sessionId` is `CLAUDE_CODE_SESSION_ID` when the
//! host was launched from a Claude Code session, and empty otherwise.

use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use fs2::FileExt;

/// First port tried. Gearmulator starts at 13710, so a host can run both.
pub(crate) const FIRST_PORT: u16 = 13_720;

/// How many consecutive ports to try when the first is taken.
pub(crate) const PORT_ATTEMPTS: u16 = 32;

pub use composesiren_mcp_api::discovery::{Instance, default_path, pid_alive};

/// Session id inherited from the environment, if any.
#[must_use]
pub fn session_id_from_env() -> String {
    std::env::var("CLAUDE_CODE_SESSION_ID").unwrap_or_default()
}

/// Insert `instance`, dropping any previous entry on the same port and any
/// entry whose process is gone.
pub fn register(path: &Path, instance: &Instance) -> io::Result<()> {
    with_locked(path, |instances| {
        instances.retain(|entry| entry.port != instance.port && pid_alive(entry.pid));
        instances.push(instance.clone());
    })
}

/// Remove the entry for `port`.
pub fn unregister(path: &Path, port: u16) -> io::Result<()> {
    with_locked(path, |instances| {
        instances.retain(|entry| entry.port != port && pid_alive(entry.pid));
    })
}

fn with_locked(path: &Path, edit: impl FnOnce(&mut Vec<Instance>)) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock_exclusive()?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let mut instances = if text.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str::<Vec<Instance>>(&text).unwrap_or_default()
    };
    edit(&mut instances);
    let body = serde_json::to_string_pretty(&instances)?;
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(body.as_bytes())?;
    file.write_all(b"\n")?;
    file.unlock()?;
    Ok(())
}

/// Current process id.
#[must_use]
pub fn current_pid() -> u32 {
    std::process::id()
}

#[cfg(test)]
mod tests {
    use super::{Instance, register, unregister};

    fn sample(port: u16) -> Instance {
        Instance {
            plugin_name: "OneSiren".to_owned(),
            plugin_4cc: "MvOS".to_owned(),
            port,
            pid: current_pid_for_test(),
            session_id: String::new(),
            standalone: true,
        }
    }

    fn current_pid_for_test() -> u32 {
        std::process::id()
    }

    #[test]
    fn register_then_unregister_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "composesiren-mcp-discovery-{}-{}",
            std::process::id(),
            port_salt()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".composesiren_mcp.json");

        register(&path, &sample(13720)).unwrap();
        register(&path, &sample(13721)).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let parsed: Vec<Instance> = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].plugin_4cc, "MvOS");
        assert!(parsed[1].standalone);

        unregister(&path, 13720).unwrap();
        let parsed: Vec<Instance> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].port, 13721);

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn port_salt() -> u32 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(1)
    }
}
