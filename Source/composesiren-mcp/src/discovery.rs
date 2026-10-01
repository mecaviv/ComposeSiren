//! Where the server lists itself: ComposeSiren's discovery file (`composesiren-mcp-api`), kept
//! by `mcp-discovery` under a file lock. Each running plugin or standalone appends itself when
//! its server binds a port, and removes itself when the server stops. `sessionId` is
//! `CLAUDE_CODE_SESSION_ID` when the host was launched from a Claude Code session, and empty
//! otherwise.

pub use composesiren_mcp_api::discovery::{Instance, Registry, current_pid, registry, session_id_from_env};

/// First port tried. Gearmulator starts at 13710, so a host can run both.
pub(crate) const FIRST_PORT: u16 = 13_720;

/// How many consecutive ports to try when the first is taken.
pub(crate) const PORT_ATTEMPTS: u16 = 32;
