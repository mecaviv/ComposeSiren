//! What ComposeSiren's MCP server and its clients agree on.
//!
//! The server (`composesiren-mcp`) builds its tools from these types, and a
//! client (`tap-viewer`, franz...) sends and reads the same ones, so a change
//! to a tool is a compile error on both sides instead of a silent mismatch.
//!
//! - [`tool`]: the tools' names.
//! - [`args`]: the arguments each tool takes.
//! - [`replies`]: the replies clients read.
//! - [`discovery`]: `~/.composesiren_mcp.json`, where running instances list themselves.
//! - `client` (feature `client`): a blocking client of the server.

pub mod args;
pub mod discovery;
pub mod replies;
pub mod tool;

#[cfg(feature = "client")]
pub mod client;
