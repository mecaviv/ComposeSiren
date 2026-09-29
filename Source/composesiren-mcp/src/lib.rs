//! In-process MCP server for ComposeSiren.
//!
//! The plugin links this static library and calls [`ffi`]. The server listens
//! on `127.0.0.1` and writes `~/.composesiren_mcp.json` so a client can find it,
//! the same arrangement Gearmulator uses for `~/.gearmulator_mcp.json`.

#![allow(unsafe_code, reason = "the public surface is a C ABI")]

mod discovery;
mod dispatch;
pub mod ffi;
mod server;
mod tools;
