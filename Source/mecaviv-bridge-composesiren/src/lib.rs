//! ComposeSiren's park bridge: what SirenOrchestra links to drive the sirens.
//!
//! It lives in the ComposeSiren repository because it is made for this
//! plugin: its API is the one `SirenUdpBridge.h` expects. The protocol comes
//! from the `mecaviv-v1` crate of mecaviv-rs, checked out next to ComposeSiren.
//!
//! On enable it tries `mecaviv-bridge-daemon` on the local socket and falls
//! back to the in-process V1 path (`SirenLink`'s direct UDP) when the
//! handshake fails. [`Bridge::with_park`] never talks to the daemon. Its API,
//! and the C ABI and C++ wrapper in `include/`, stay the same except for
//! [`Backend`], which the UI uses for the "Sirenes physiques" tooltip.
//!
//! - [`Bridge`]: the handle. Disabled until [`Bridge::set_enabled`], so an
//!   instance in the studio never talks to a park that is not there.
//! - [`ParkTable`]: where the sirens and their drives are.
//! - [`ffi`]: the C ABI. `build.rs` generates its header,
//!   `include/mecaviv_bridge.h`, with cbindgen. `include/mecaviv_bridge.hpp`
//!   is the hand-written C++ wrapper on top of it.

mod bridge;
mod daemon;
pub mod ffi;
mod link;
mod park;

pub use bridge::{Backend, Bridge};
pub use mecaviv_v1::SirenId;
pub use mecaviv_v1::keb::DriveState;
pub use park::{ParkTable, SirenEndpoints};

/// The library's version, reported through the C ABI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
