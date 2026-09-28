//! The bridge handle and its worker thread.
//!
//! Callers never touch a socket. The audio thread pushes MIDI into a bounded
//! lock-free queue. Other threads set atomics (enable, resets, ST). A worker
//! thread owned by the bridge does all the network I/O.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mecaviv_v1::SirenId;
use mecaviv_v1::keb::DriveState;

use crate::link::Link;
use crate::park::ParkTable;

/// MIDI messages that can wait for the worker. Beyond that, pushes fail.
const MIDI_QUEUE: usize = 1024;
/// How often the worker drains the MIDI queue while enabled.
const TICK: Duration = Duration::from_millis(1);
/// How long the worker sleeps while disabled. Control calls wake it.
const IDLE: Duration = Duration::from_millis(100);
/// How often the drives are polled.
const POLL: Duration = Duration::from_secs(1);

/// No ST request pending.
const ST_NONE: i8 = -1;

/// A connection to the park.
///
/// Disabled when created: nothing is sent and no drive is polled until
/// [`Bridge::set_enabled`]. Enabling sends the reset handshake to every siren.
/// Disabling drops what is queued, and forgets the drive states.
///
/// Every method takes `&self` and can be called from any thread.
/// [`Bridge::push_midi`] is the one meant for the audio thread: it neither
/// blocks, allocates, nor touches a socket.
pub struct Bridge {
    shared: Arc<Shared>,
    midi: SyncSender<[u8; 3]>,
    worker: Option<JoinHandle<()>>,
}

struct Shared {
    enabled: AtomicBool,
    stop: AtomicBool,
    /// Sirens to reset, bit 0 = S1.
    resets: AtomicU8,
    /// ST request: 0 off, 1 on, or [`ST_NONE`].
    st: AtomicI8,
    /// Drive states for readers, S1 first.
    states: [AtomicI8; 7],
}

impl Bridge {
    /// A bridge to the park at its default addresses.
    ///
    /// # Errors
    ///
    /// If the sockets cannot be created, or the worker thread cannot start.
    pub fn new() -> io::Result<Self> {
        Self::with_park(ParkTable::defaults())
    }

    /// A bridge to the park described by `park`.
    ///
    /// # Errors
    ///
    /// If the sockets cannot be created, or the worker thread cannot start.
    pub fn with_park(park: ParkTable) -> io::Result<Self> {
        let link = Link::new(park)?;
        let shared = Arc::new(Shared {
            enabled: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            resets: AtomicU8::new(0),
            st: AtomicI8::new(ST_NONE),
            states: std::array::from_fn(|_| AtomicI8::new(state_code(DriveState::Unknown))),
        });
        let (midi, queue) = mpsc::sync_channel(MIDI_QUEUE);
        let worker = thread::Builder::new()
            .name("mecaviv-bridge-composesiren".into())
            .spawn({
                let shared = Arc::clone(&shared);
                move || run(&shared, &queue, link)
            })?;
        Ok(Self {
            shared,
            midi,
            worker: Some(worker),
        })
    }

    /// Drives the park or not.
    pub fn set_enabled(&self, enabled: bool) {
        self.shared.enabled.store(enabled, Ordering::Release);
        self.wake();
    }

    /// Whether the park is driven.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.shared.enabled.load(Ordering::Acquire)
    }

    /// Queues a 3-byte MIDI message. Real-time safe.
    ///
    /// Returns `false` if the bridge is disabled or the queue is full. Messages
    /// the park does not accept are dropped when sent, not here.
    ///
    /// The queue is a bounded `std::sync::mpsc` channel. Its `try_send` is a
    /// compare-and-swap into a preallocated ring, and it only takes a lock to
    /// wake a receiver blocked on it. The worker never blocks on the queue (it
    /// polls it with `try_recv`), so this never locks.
    #[allow(
        clippy::must_use_candidate,
        reason = "a full queue drops the message, like the C++ FIFO did"
    )]
    pub fn push_midi(&self, bytes: [u8; 3]) -> bool {
        self.is_enabled() && self.midi.try_send(bytes).is_ok()
    }

    /// Resets `siren`: closes its flaps and stops its motor.
    pub fn reset(&self, siren: SirenId) {
        self.shared
            .resets
            .fetch_or(1 << (siren.get() - 1), Ordering::AcqRel);
        self.wake();
    }

    /// Resets every siren.
    pub fn reset_all(&self) {
        self.shared.resets.fetch_or(0x7F, Ordering::AcqRel);
        self.wake();
    }

    /// Enables (ST on) or disables every siren's drive.
    pub fn set_st_all(&self, enabled: bool) {
        self.shared.st.store(i8::from(enabled), Ordering::Release);
        self.wake();
    }

    /// The drive state of `siren`, as last read from its drive.
    #[must_use]
    pub fn st_state(&self, siren: SirenId) -> DriveState {
        state_from_code(self.shared.states[usize::from(siren.get() - 1)].load(Ordering::Acquire))
    }

    fn wake(&self) {
        if let Some(worker) = &self.worker {
            worker.thread().unpark();
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

fn run(shared: &Shared, queue: &Receiver<[u8; 3]>, mut link: Link) {
    let mut was_enabled = false;
    let mut last_poll: Option<Instant> = None;

    while !shared.stop.load(Ordering::Acquire) {
        let enabled = shared.enabled.load(Ordering::Acquire);
        if enabled != was_enabled {
            was_enabled = enabled;
            if enabled {
                link.reset_all();
                last_poll = None;
            } else {
                link.forget_drive_states();
                publish_states(shared, &link);
            }
        }

        if enabled {
            let resets = shared.resets.swap(0, Ordering::AcqRel);
            for siren in SirenId::all() {
                if resets & (1 << (siren.get() - 1)) != 0 {
                    link.reset(siren);
                }
            }
            match shared.st.swap(ST_NONE, Ordering::AcqRel) {
                ST_NONE => {}
                st => link.st_all(st != 0),
            }
            while let Ok(bytes) = queue.try_recv() {
                link.midi(bytes);
            }
            link.receive_drive_replies();
            if last_poll.is_none_or(|at| at.elapsed() >= POLL) {
                link.poll_drives();
                last_poll = Some(Instant::now());
            }
            publish_states(shared, &link);
        } else {
            // Nothing queued while disabled is sent later.
            shared.resets.store(0, Ordering::Release);
            shared.st.store(ST_NONE, Ordering::Release);
            while queue.try_recv().is_ok() {}
        }

        thread::park_timeout(if enabled { TICK } else { IDLE });
    }
}

fn publish_states(shared: &Shared, link: &Link) {
    for siren in SirenId::all() {
        shared.states[usize::from(siren.get() - 1)]
            .store(state_code(link.drive_state(siren)), Ordering::Release);
    }
}

const fn state_code(state: DriveState) -> i8 {
    match state {
        DriveState::Unknown => -1,
        DriveState::Disabled => 0,
        DriveState::Enabled => 1,
    }
}

const fn state_from_code(code: i8) -> DriveState {
    match code {
        0 => DriveState::Disabled,
        1 => DriveState::Enabled,
        _ => DriveState::Unknown,
    }
}
