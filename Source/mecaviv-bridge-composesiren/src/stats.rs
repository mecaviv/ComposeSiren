//! What this bridge sent to `mecaviv-bridge-daemon`, for the About dialog.
//!
//! Plain atomics: the worker thread increments, the UI thread reads. Counts
//! start with the bridge and survive enable/disable and fallbacks.

use std::sync::atomic::{AtomicU64, Ordering};

/// Counters shared between the worker and [`crate::Bridge::daemon_stats`].
#[derive(Default)]
pub(crate) struct DaemonCounters {
    pub(crate) sessions: AtomicU64,
    pub(crate) session_failures: AtomicU64,
    pub(crate) midi: AtomicU64,
    pub(crate) midi_ignored: AtomicU64,
    pub(crate) resets: AtomicU64,
    pub(crate) reset_all: AtomicU64,
    pub(crate) st_all: AtomicU64,
    pub(crate) drive_states: AtomicU64,
}

impl DaemonCounters {
    pub(crate) fn add(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> DaemonStats {
        let get = |c: &AtomicU64| c.load(Ordering::Relaxed);
        DaemonStats {
            sessions: get(&self.sessions),
            session_failures: get(&self.session_failures),
            midi: get(&self.midi),
            midi_ignored: get(&self.midi_ignored),
            resets: get(&self.resets),
            reset_all: get(&self.reset_all),
            st_all: get(&self.st_all),
            drive_states: get(&self.drive_states),
        }
    }
}

/// Calls made to the daemon since the bridge was created.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DaemonStats {
    /// Sessions opened: hello accepted and first reset sent.
    pub sessions: u64,
    /// Attempts that failed (socket, handshake, or first write).
    pub session_failures: u64,
    /// MIDI messages sent.
    pub midi: u64,
    /// MIDI messages the daemon's grammar drops (other statuses or channels).
    pub midi_ignored: u64,
    /// Resets of one siren.
    pub resets: u64,
    /// Resets of every siren.
    pub reset_all: u64,
    /// ST of every siren.
    pub st_all: u64,
    /// Drive-state records received.
    pub drive_states: u64,
}
