//! A blocking client of ComposeSiren's MCP server: `mcp-discovery`'s, pointed at ComposeSiren's
//! discovery file.

pub use mcp_discovery::client::Client;

use crate::discovery::registry;
use crate::tool;

/// Every running instance (on `port`, if given) that offers `tool`. None when ComposeSiren is
/// not running, with no error: for what is only done if it can be.
#[must_use]
pub fn connect_all(port: Option<u16>, tool: &str) -> Vec<Client> {
    Client::connect_all(&registry(), port, tool)
}

/// The instance on `port`, or the first running one that offers `tool` (`missing` says why
/// one might not).
pub fn connect_with(port: Option<u16>, tool: &str, missing: &str) -> Result<Client, String> {
    Client::connect_with(&registry(), port, tool, missing)
}

/// The instance on `port`, or the first running one that can record.
pub fn connect(port: Option<u16>) -> Result<Client, String> {
    connect_with(port, tool::START_RECORDING, "built without COMPOSESIREN_RECORD")
}
