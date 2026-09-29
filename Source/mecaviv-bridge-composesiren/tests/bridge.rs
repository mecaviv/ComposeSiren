//! The bridge against a fake park on localhost: one socket for every card, one
//! per drive.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use mecaviv_bridge_composesiren::{
    Backend, Bridge, DriveState, ParkTable, SirenEndpoints, SirenId,
};
use mecaviv_v1::frame::Frame;
use mecaviv_v1::keb::{KebModel, StRequest};
use mecaviv_v1::value::U7;
use mecaviv_v1::{Channel, ChannelMessage, Command, Device, Target};

struct FakePark {
    cards: UdpSocket,
    drives: Vec<UdpSocket>,
    table: ParkTable,
}

impl FakePark {
    fn new() -> Self {
        let socket = || {
            let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_millis(500)))
                .unwrap();
            socket
        };
        let cards = socket();
        let drives: Vec<_> = (0..7).map(|_| socket()).collect();
        let table = ParkTable {
            sirens: std::array::from_fn(|i| SirenEndpoints {
                card: cards.local_addr().unwrap(),
                drive: drives[i].local_addr().unwrap(),
                model: if i == 3 { KebModel::F6 } else { KebModel::F5 },
            }),
        };
        Self {
            cards,
            drives,
            table,
        }
    }

    fn next_card_command(&self) -> Option<Command> {
        let mut buf = [0; 64];
        let len = self.cards.recv(&mut buf).ok()?;
        Some(Command::decode(&Frame::parse(&buf[..len]).unwrap()).unwrap())
    }

    fn next_drive_request(&self, siren: u8) -> (Vec<u8>, SocketAddr) {
        let mut buf = [0; 64];
        let (len, from) = self.drives[usize::from(siren - 1)]
            .recv_from(&mut buf)
            .unwrap();
        (buf[..len].to_vec(), from)
    }
}

fn siren(n: u8) -> SirenId {
    SirenId::new(n).unwrap()
}

fn wait_for(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

#[test]
fn disabled_bridge_sends_nothing() {
    let park = FakePark::new();
    let bridge = Bridge::with_park(park.table.clone()).unwrap();
    assert!(!bridge.is_enabled());
    assert!(!bridge.push_midi([0x90, 60, 100]));
    bridge.reset_all();
    bridge.set_st_all(true);
    assert_eq!(park.next_card_command(), None);
    assert_eq!(bridge.st_state(siren(1)), DriveState::Unknown);
}

#[test]
fn enabled_bridge_drives_the_park() {
    let park = FakePark::new();
    let bridge = Bridge::with_park(park.table.clone()).unwrap();

    // Enabling sends the reset handshake to every siren.
    bridge.set_enabled(true);
    for _ in 1..=7 {
        assert_eq!(park.next_card_command(), Some(Command::Reset));
    }

    // Every drive is polled, S4 with the F6 request.
    let (request, bridge_addr) = park.next_drive_request(1);
    assert_eq!(request, StRequest(KebModel::F5).encode());
    assert_eq!(
        park.next_drive_request(4).0,
        StRequest(KebModel::F6).encode()
    );

    // A note on with velocity 0 goes out as a note off, on its siren.
    assert!(bridge.push_midi([0x92, 60, 0]));
    let note_off = ChannelMessage::NoteOff {
        channel: Channel::new(2).unwrap(),
        note: U7::new(60).unwrap(),
        velocity: U7::MIN,
    };
    assert_eq!(
        park.next_card_command(),
        Some(Command::Midi {
            device: Device::Siren(siren(3)),
            message: note_off
        }),
    );

    // Channel 9 and program changes do not reach the sirens.
    assert!(bridge.push_midi([0x98, 60, 100]));
    assert!(bridge.push_midi([0xC0, 5, 0]));
    bridge.reset(siren(2));
    assert_eq!(park.next_card_command(), Some(Command::Reset));

    // ST goes to every siren as a full frame.
    bridge.set_st_all(true);
    for n in 1..=7 {
        let target = Target::Device(Device::Siren(siren(n)));
        assert_eq!(
            park.next_card_command(),
            Some(Command::St {
                enabled: true,
                target
            })
        );
    }

    // Drive replies set the states.
    let f5_on = [
        0x02, b'0', b'2', b'1', b'6', b'0', b'0', b'0', b'9', 0x03, 0x20,
    ];
    park.drives[0].send_to(&f5_on, bridge_addr).unwrap();
    let mut f6_off = vec![0x02, b'G', b'1'];
    f6_off.extend(b"00000007");
    f6_off.extend([0x03, 0x20]);
    park.drives[3].send_to(&f6_off, bridge_addr).unwrap();
    assert!(wait_for(|| bridge.st_state(siren(1)) == DriveState::Enabled));
    assert!(wait_for(
        || bridge.st_state(siren(4)) == DriveState::Disabled
    ));
    assert_eq!(bridge.st_state(siren(2)), DriveState::Unknown);

    // Disabling forgets them.
    bridge.set_enabled(false);
    assert!(wait_for(|| bridge.st_state(siren(1)) == DriveState::Unknown));
    assert!(!bridge.push_midi([0x90, 60, 100]));
}

#[test]
fn silent_drive_becomes_unknown() {
    let park = FakePark::new();
    let bridge = Bridge::with_park(park.table.clone()).unwrap();
    bridge.set_enabled(true);

    let (_, bridge_addr) = park.next_drive_request(1);
    let f5_on = [
        0x02, b'0', b'2', b'1', b'6', b'0', b'0', b'0', b'9', 0x03, 0x20,
    ];
    park.drives[0].send_to(&f5_on, bridge_addr).unwrap();
    assert!(wait_for(|| bridge.st_state(siren(1)) == DriveState::Enabled));

    // No reply to the next round: unknown at the round after.
    assert!(wait_for(|| bridge.st_state(siren(1)) == DriveState::Unknown));
}

#[test]
fn generated_header_declares_every_exported_function() {
    let ffi = include_str!("../src/ffi.rs");
    let header = include_str!("../include/mecaviv_bridge.h");
    let exported: Vec<_> = ffi
        .lines()
        .filter_map(|line| line.split("extern \"C\" fn ").nth(1))
        .filter_map(|rest| rest.split('(').next())
        .collect();
    assert!(exported.len() >= 10, "{exported:?}");
    for name in exported {
        assert!(
            header.contains(&format!("{name}(")),
            "{name} is not declared"
        );
    }
}

#[test]
fn with_park_is_in_process() {
    let park = FakePark::new();
    let bridge = Bridge::with_park(park.table.clone()).unwrap();
    assert_eq!(bridge.backend(), Backend::InProcess);
    assert_eq!(bridge.backend().tooltip(), "Uses the in-process bridge.");
    bridge.set_enabled(true);
    assert!(wait_for(|| bridge.backend() == Backend::InProcess));
}

#[test]
#[allow(unsafe_code, reason = "reads the C string the ABI returns")]
fn version_is_the_crate_version() {
    // SAFETY: the ABI returns a static NUL-terminated string.
    let version = unsafe {
        std::ffi::CStr::from_ptr(mecaviv_bridge_composesiren::ffi::mecaviv_bridge_version())
    };
    assert_eq!(version.to_str(), Ok(mecaviv_bridge_composesiren::VERSION));
}

#[test]
#[allow(unsafe_code, reason = "reads the C string the ABI returns")]
fn backend_tooltip_matches_backend() {
    let park = FakePark::new();
    let bridge = Bridge::with_park(park.table.clone()).unwrap();
    // SAFETY: the ABI returns a static NUL-terminated string.
    let tip = unsafe {
        std::ffi::CStr::from_ptr(
            mecaviv_bridge_composesiren::ffi::mecaviv_bridge_backend_tooltip(std::ptr::null()),
        )
    };
    assert_eq!(tip.to_str(), Ok(Backend::None.tooltip()));
    let _ = bridge;
}

#[cfg(unix)]
mod daemon {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};

    use mecaviv_bridge_daemon::{
        ClientId, ClientMessage, ServerMessage, read_record, write_record,
    };
    use mecaviv_v1::Command;

    static SOCK: AtomicU64 = AtomicU64::new(0);

    struct Mock {
        path: std::path::PathBuf,
        received: Receiver<ClientMessage>,
        _thread: JoinHandle<()>,
    }

    impl Drop for Mock {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn spawn_mock() -> Mock {
        let path = std::env::temp_dir().join(format!(
            "mecaviv-cs-bridge-{}-{}.sock",
            std::process::id(),
            SOCK.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let (tx, received) = mpsc::channel();
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                while let Ok(payload) = read_record(&mut stream) {
                    let Ok(message) = ClientMessage::decode(&payload) else {
                        break;
                    };
                    match &message {
                        ClientMessage::Hello(_) => {
                            let welcome = ServerMessage::Welcome {
                                client: ClientId(1),
                            };
                            if write_record(&mut stream, &welcome.encode()).is_err() {
                                break;
                            }
                        }
                        ClientMessage::Subscribe { .. } => {
                            let state = ServerMessage::DriveState {
                                siren: super::siren(1),
                                state: DriveState::Enabled,
                            };
                            if write_record(&mut stream, &state.encode()).is_err() {
                                break;
                            }
                        }
                        ClientMessage::Park { .. } => {
                            if tx.send(message).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });
        Mock {
            path,
            received,
            _thread: thread,
        }
    }

    #[test]
    fn new_uses_the_daemon_when_it_accepts_hello() {
        let mock = spawn_mock();
        let bridge = Bridge::with_socket(mock.path.clone()).unwrap();
        assert_eq!(bridge.backend(), Backend::Daemon);
        assert_eq!(
            bridge.backend().tooltip(),
            "Uses the mecaviv-bridge daemon."
        );

        bridge.set_enabled(true);
        assert!(wait_for(|| bridge.backend() == Backend::Daemon));

        let reset = mock
            .received
            .recv_timeout(Duration::from_secs(3))
            .expect("reset handshake");
        match reset {
            ClientMessage::Park {
                dest,
                command: Command::Reset,
                ..
            } => assert_eq!(dest, Target::All),
            other => panic!("{other:?}"),
        }

        assert!(wait_for(|| bridge.st_state(siren(1)) == DriveState::Enabled));
        assert!(bridge.push_midi([0x90, 60, 100]));
        let midi = mock
            .received
            .recv_timeout(Duration::from_secs(3))
            .expect("midi");
        match midi {
            ClientMessage::Park {
                command: Command::Midi { .. },
                ..
            } => {}
            other => panic!("{other:?}"),
        }

        bridge.set_enabled(false);
        assert!(wait_for(|| bridge.st_state(siren(1)) == DriveState::Unknown));
        assert_eq!(bridge.backend(), Backend::Daemon);
    }

    #[test]
    fn with_park_does_not_talk_to_the_daemon() {
        let mock = spawn_mock();
        let park = FakePark::new();
        let bridge = Bridge::with_park(park.table.clone()).unwrap();
        assert_eq!(bridge.backend(), Backend::InProcess);
        bridge.set_enabled(true);
        for _ in 1..=7 {
            assert_eq!(park.next_card_command(), Some(Command::Reset));
        }
        assert!(
            mock.received
                .recv_timeout(Duration::from_millis(200))
                .is_err(),
            "with_park must not send Park records"
        );
    }
}
