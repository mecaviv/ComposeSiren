//! The bridge handle and its worker thread.
//!
//! Callers never touch a socket. The audio thread pushes MIDI into a bounded
//! lock-free queue. Other threads set atomics (enable, resets, ST). A worker
//! thread owned by the bridge does all the network I/O.
//!
//! [`Bridge::new`] tries `mecaviv-bridge-daemon` on the local socket and falls
//! back to the in-process [`Link`]. [`Bridge::with_park`] never talks to the
//! daemon, so tests that bind a fake park are not stolen by a running daemon.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mecaviv_bridge_daemon::ServerMessage;
use mecaviv_bridge_daemon::socket_path;
use mecaviv_v1::SirenId;
use mecaviv_v1::keb::DriveState;

use crate::daemon::{self, Client};
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

const BACKEND_NONE: u8 = 0;
const BACKEND_IN_PROCESS: u8 = 1;
const BACKEND_DAEMON: u8 = 2;

/// How the park is driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// The bridge is enabled but has no transport, or the handle is null.
    None,
    /// This process talks to the boards, as `SirenLink` did.
    InProcess,
    /// The park daemon owns the hardware link.
    Daemon,
}

impl Backend {
    /// Tooltip for the "Sirenes physiques" control.
    #[must_use]
    pub const fn tooltip(self) -> &'static str {
        match self {
            Self::None => "Park bridge is not available.",
            Self::InProcess => "Uses the in-process bridge.",
            Self::Daemon => "Uses the mecaviv-bridge daemon.",
        }
    }

    const fn from_code(code: u8) -> Self {
        match code {
            BACKEND_DAEMON => Self::Daemon,
            BACKEND_IN_PROCESS => Self::InProcess,
            _ => Self::None,
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::None => BACKEND_NONE,
            Self::InProcess => BACKEND_IN_PROCESS,
            Self::Daemon => BACKEND_DAEMON,
        }
    }
}

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
    /// [`Backend::code`] while enabled; 0 while disabled.
    backend: AtomicU8,
    /// Try this socket on enable. `None` for [`Bridge::with_park`].
    daemon_socket: Option<PathBuf>,
}

impl Bridge {
    /// A bridge to the park at its default addresses.
    ///
    /// On enable it tries the daemon at [`socket_path`]. If the handshake
    /// fails, it drives the default park itself.
    ///
    /// # Errors
    ///
    /// If the worker thread cannot start.
    pub fn new() -> io::Result<Self> {
        Self::with_socket(socket_path())
    }

    /// Like [`Bridge::new`], but the daemon is at `socket`.
    ///
    /// # Errors
    ///
    /// If the worker thread cannot start.
    pub fn with_socket(socket: PathBuf) -> io::Result<Self> {
        Self::spawn(Some(socket), None)
    }

    /// A bridge to the park described by `park`. Never talks to the daemon.
    ///
    /// # Errors
    ///
    /// If the sockets cannot be created, or the worker thread cannot start.
    pub fn with_park(park: ParkTable) -> io::Result<Self> {
        Self::spawn(None, Some(Link::new(park)?))
    }

    fn spawn(daemon_socket: Option<PathBuf>, link: Option<Link>) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            enabled: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            resets: AtomicU8::new(0),
            st: AtomicI8::new(ST_NONE),
            states: std::array::from_fn(|_| AtomicI8::new(state_code(DriveState::Unknown))),
            backend: AtomicU8::new(BACKEND_NONE),
            daemon_socket,
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

    /// How the park is driven.
    ///
    /// While enabled, this is the transport the worker actually opened. While
    /// disabled, a listening daemon socket is reported as [`Backend::Daemon`]
    /// so the tooltip can say what enabling will use.
    #[must_use]
    pub fn backend(&self) -> Backend {
        if self.is_enabled() {
            return Backend::from_code(self.shared.backend.load(Ordering::Acquire));
        }
        match self.shared.daemon_socket.as_deref() {
            Some(path) if daemon::available(path) => Backend::Daemon,
            Some(_) | None => Backend::InProcess,
        }
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

fn run(shared: &Shared, queue: &Receiver<[u8; 3]>, mut link: Option<Link>) {
    let mut was_enabled = false;
    let mut last_poll: Option<Instant> = None;
    let mut daemon: Option<Client> = None;

    while !shared.stop.load(Ordering::Acquire) {
        let enabled = shared.enabled.load(Ordering::Acquire);
        if enabled != was_enabled {
            was_enabled = enabled;
            if enabled {
                start_transport(shared, &mut daemon, &mut link);
                last_poll = None;
            } else {
                daemon = None;
                if let Some(link) = link.as_mut() {
                    link.forget_drive_states();
                }
                forget_states(shared);
                shared.backend.store(BACKEND_NONE, Ordering::Release);
            }
        }

        if enabled {
            let daemon_dead = if let Some(client) = daemon.as_mut() {
                drive_daemon(shared, queue, client)
            } else {
                if let Some(link) = link.as_mut() {
                    drive_link(shared, queue, link, &mut last_poll);
                }
                false
            };
            if daemon_dead {
                fallback_to_link(shared, &mut daemon, &mut link);
            }
        } else {
            // Nothing queued while disabled is sent later.
            shared.resets.store(0, Ordering::Release);
            shared.st.store(ST_NONE, Ordering::Release);
            while queue.try_recv().is_ok() {}
        }

        thread::park_timeout(if enabled { TICK } else { IDLE });
    }
}

fn start_transport(shared: &Shared, daemon: &mut Option<Client>, link: &mut Option<Link>) {
    *daemon = None;
    if let Some(path) = shared.daemon_socket.as_deref() {
        if let Ok(mut client) = Client::connect(path) {
            if client.reset_all().is_ok() {
                *daemon = Some(client);
                shared
                    .backend
                    .store(Backend::Daemon.code(), Ordering::Release);
                return;
            }
        }
    }
    fallback_to_link(shared, daemon, link);
    if matches!(
        Backend::from_code(shared.backend.load(Ordering::Acquire)),
        Backend::InProcess
    ) {
        if let Some(link) = link.as_mut() {
            link.reset_all();
        }
    }
}

/// Drops the daemon and uses the in-process link. Returns after publishing
/// [`Backend::None`] if the sockets cannot be created.
fn fallback_to_link(shared: &Shared, daemon: &mut Option<Client>, link: &mut Option<Link>) {
    *daemon = None;
    if link.is_none() {
        *link = Link::new(ParkTable::defaults()).ok();
    }
    if link.is_some() {
        shared
            .backend
            .store(Backend::InProcess.code(), Ordering::Release);
    } else {
        shared
            .backend
            .store(Backend::None.code(), Ordering::Release);
    }
}

/// `true` when the daemon session is dead and the worker should fall back.
fn drive_daemon(shared: &Shared, queue: &Receiver<[u8; 3]>, client: &mut Client) -> bool {
    let resets = shared.resets.swap(0, Ordering::AcqRel);
    for siren in SirenId::all() {
        if resets & (1 << (siren.get() - 1)) != 0 && client.reset(siren).is_err() {
            return true;
        }
    }
    match shared.st.swap(ST_NONE, Ordering::AcqRel) {
        ST_NONE => {}
        st if client.st_all(st != 0).is_err() => return true,
        _ => {}
    }
    while let Ok(bytes) = queue.try_recv() {
        if client.midi(bytes).is_err() {
            return true;
        }
    }
    match client.poll() {
        Ok(messages) => {
            for message in messages {
                if let ServerMessage::DriveState { siren, state } = message {
                    shared.states[usize::from(siren.get() - 1)]
                        .store(state_code(state), Ordering::Release);
                }
            }
            false
        }
        Err(_) => true,
    }
}

fn drive_link(
    shared: &Shared,
    queue: &Receiver<[u8; 3]>,
    link: &mut Link,
    last_poll: &mut Option<Instant>,
) {
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
        *last_poll = Some(Instant::now());
    }
    publish_states(shared, link);
}

fn publish_states(shared: &Shared, link: &Link) {
    for siren in SirenId::all() {
        shared.states[usize::from(siren.get() - 1)]
            .store(state_code(link.drive_state(siren)), Ordering::Release);
    }
}

fn forget_states(shared: &Shared) {
    for state in &shared.states {
        state.store(state_code(DriveState::Unknown), Ordering::Release);
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
