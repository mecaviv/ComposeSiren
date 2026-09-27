//! ComposeSiren's park bridge: what SirenOrchestra links to drive the sirens.
//!
//! It lives in the ComposeSiren repository because it is made for this
//! plugin: its API is the one `SirenUdpBridge.h` expects. The protocol comes
//! from the `mecaviv-v1` crate of mecaviv-rs, checked out next to ComposeSiren.
//!
//! Today it drives the sirens in-process, over the V1 direct path, as
//! `SirenLink` did. Later the same [`Bridge`] will talk to
//! `mecaviv-bridge-daemon` when it runs, and fall back to driving the park
//! itself when it does not. Its API, and the C ABI and C++ wrapper in
//! `include/`, stay the same.
//!
//! - [`Bridge`]: the handle. Disabled until [`Bridge::set_enabled`], so an
//!   instance in the studio never talks to a park that is not there.
//! - [`ParkTable`]: where the sirens and their drives are.
//! - [`ffi`]: the C ABI. `build.rs` generates its header,
//!   `include/mecaviv_bridge.h`, with cbindgen. `include/mecaviv_bridge.hpp`
//!   is the hand-written C++ wrapper on top of it.

mod bridge;
pub mod ffi;
mod link;
mod park;

pub use bridge::Bridge;
pub use mecaviv_v1::SirenId;
pub use mecaviv_v1::keb::DriveState;
pub use park::{ParkTable, SirenEndpoints};

/// The library's version, reported to the daemon in the client handshake.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
