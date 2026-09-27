//! The hardware link: the frames the bridge sends to the sirens and their
//! drives, over the V1 direct path (spec §2.3, §16).
//!
//! Every method is synchronous and non-blocking. The sockets are never
//! connected: each datagram goes out with `send_to`, so a network that comes
//! up late does not break the session.

use std::io;
use std::net::{Ipv4Addr, UdpSocket};

use mecaviv_v1::keb::{self, DriveState, StRequest};
use mecaviv_v1::{ChannelMessage, Command, Device, SirenId, Target};

use crate::park::ParkTable;

pub(crate) struct Link {
    park: ParkTable,
    /// Sends to the cards.
    cards: UdpSocket,
    /// Sends to the drives, and receives their replies on its port.
    drives: UdpSocket,
    states: [DriveState; 7],
    /// Sirens polled since the last round without a reply, bit 0 = S1.
    awaiting: u8,
}

impl Link {
    pub(crate) fn new(park: ParkTable) -> io::Result<Self> {
        let socket = || {
            let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
            socket.set_nonblocking(true)?;
            Ok::<_, io::Error>(socket)
        };
        Ok(Self {
            park,
            cards: socket()?,
            drives: socket()?,
            states: [DriveState::Unknown; 7],
            awaiting: 0,
        })
    }

    fn send(&self, siren: SirenId, command: Command) {
        // Best effort, like every V1 client: a lost datagram is not retried.
        let _ = self
            .cards
            .send_to(command.encode().as_bytes(), self.park.siren(siren).card);
    }

    /// Sends a MIDI message to the siren of its channel. Only note off, note
    /// on, control change and pitch bend on channels 1..=7 are sent, as with
    /// `SirenLink`. A note on with velocity 0 is sent as a note off.
    pub(crate) fn midi(&self, bytes: [u8; 3]) -> bool {
        let Ok(message) = ChannelMessage::from_bytes(bytes) else {
            return false;
        };
        let Some(siren) = SirenId::new(message.channel().number()) else {
            return false;
        };
        let message = match message {
            ChannelMessage::NoteOn {
                channel,
                note,
                velocity,
            } if velocity.get() == 0 => ChannelMessage::NoteOff {
                channel,
                note,
                velocity,
            },
            other => other,
        };
        self.send(
            siren,
            Command::Midi {
                device: Device::Siren(siren),
                message,
            },
        );
        true
    }

    /// The handshake, and the reset button: closes the flaps, stops the motor.
    pub(crate) fn reset(&self, siren: SirenId) {
        self.send(siren, Command::Reset);
    }

    pub(crate) fn reset_all(&self) {
        SirenId::all().for_each(|siren| self.reset(siren));
    }

    pub(crate) fn st_all(&self, enabled: bool) {
        for siren in SirenId::all() {
            let target = Target::Device(Device::Siren(siren));
            self.send(siren, Command::St { enabled, target });
        }
    }

    /// Sends a state read to every drive. Sirens that did not answer the
    /// previous round become unknown.
    pub(crate) fn poll_drives(&mut self) {
        for siren in SirenId::all() {
            let bit = 1 << (siren.get() - 1);
            if self.awaiting & bit != 0 {
                self.states[usize::from(siren.get() - 1)] = DriveState::Unknown;
            }
            let drive = self.park.siren(siren);
            if self
                .drives
                .send_to(StRequest(drive.model).encode(), drive.drive)
                .is_ok()
            {
                self.awaiting |= bit;
            } else {
                self.awaiting &= !bit;
            }
        }
    }

    /// Reads every drive reply waiting on the socket.
    pub(crate) fn receive_drive_replies(&mut self) {
        let mut reply = [0; 32];
        while let Ok((len, from)) = self.drives.recv_from(&mut reply) {
            let Some(siren) = self.park.siren_with_drive(from) else {
                continue;
            };
            if let Some(state) = keb::parse_st_reply(&reply[..len]) {
                self.states[usize::from(siren.get() - 1)] = state;
                self.awaiting &= !(1 << (siren.get() - 1));
            }
        }
    }

    pub(crate) fn drive_state(&self, siren: SirenId) -> DriveState {
        self.states[usize::from(siren.get() - 1)]
    }

    pub(crate) fn forget_drive_states(&mut self) {
        self.states = [DriveState::Unknown; 7];
        self.awaiting = 0;
    }
}
