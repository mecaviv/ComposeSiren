//! `~/.composesiren_mcp.json`: where each running ComposeSiren instance lists itself.
//!
//! The file, its entries and the locking are `mcp-discovery`'s; this module only says which
//! file is ComposeSiren's.

use std::path::PathBuf;

pub use mcp_discovery::{Instance, Registry, current_pid, pid_alive, session_id_from_env};

/// The file's name, in the home folder.
pub const FILE_NAME: &str = ".composesiren_mcp.json";

/// ComposeSiren's discovery file: `~/.composesiren_mcp.json`, or `%USERPROFILE%\.composesiren_mcp.json`.
#[must_use]
pub fn registry() -> Registry {
    Registry::in_home(FILE_NAME)
}

/// Where that file is.
#[must_use]
pub fn default_path() -> PathBuf {
    registry().path().to_path_buf()
}

/// The ComposeSiren instances whose process still exists.
pub fn running() -> Result<Vec<Instance>, String> {
    registry().running()
}

#[cfg(test)]
mod tests {
    use super::{FILE_NAME, Instance, default_path};

    #[test]
    fn reads_the_file_the_server_writes() {
        let text = r#"[{"pluginName":"SirenOrchestra","plugin4CC":"MvSO","port":13720,"pid":1,"sessionId":"","standalone":true}]"#;
        let instances: Vec<Instance> = serde_json::from_str(text).unwrap();
        assert_eq!(instances[0].plugin_4cc, "MvSO");
        assert!(instances[0].standalone);
    }

    #[test]
    fn the_file_is_in_the_home_folder() {
        assert!(default_path().ends_with(FILE_NAME));
    }
}
