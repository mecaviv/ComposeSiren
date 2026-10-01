//! How MCP servers of the mecaviv projects list themselves, and how clients find them.
//!
//! Each running server (a plugin, a standalone, a daemon) appends an [`Instance`] to its
//! application's discovery file when it binds a port, and removes it when it stops. The file
//! is a JSON list in the shape Gearmulator uses for `~/.gearmulator_mcp.json`; one file per
//! application (`~/.composesiren_mcp.json`...), so this crate knows no application: it takes
//! the file's name.
//!
//! - [`Registry`]: the file. Servers [`register`](Registry::register) and
//!   [`unregister`](Registry::unregister) under a file lock; anyone can
//!   [`read`](Registry::read) it or list the [`running`](Registry::running) instances.
//! - `client` (feature `client`): a blocking MCP client that opens a session with an
//!   instance and calls its tools.
//!
//! An application declares its own tools, arguments and replies on top (for ComposeSiren,
//! `composesiren-mcp-api`).

mod registry;

pub use registry::{Instance, Registry, current_pid, pid_alive, session_id_from_env};

#[cfg(feature = "client")]
pub mod client;
