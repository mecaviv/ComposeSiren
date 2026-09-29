//! Unix local-socket client.

use std::io::{self, Read};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use mecaviv_bridge_daemon::{
    ClientMessage, Hello, MAX_PAYLOAD, RequestId, ServerMessage, read_record, write_record,
};
use mecaviv_v1::SirenId;

/// How long the handshake waits for [`ServerMessage::Welcome`].
const HANDSHAKE: Duration = Duration::from_millis(500);

/// A live session with the daemon, after hello and a drive-state subscription.
pub(crate) struct Client {
    stream: UnixStream,
    next_request: u64,
}

impl Client {
    /// Connects, sends hello, and subscribes to drive states.
    ///
    /// # Errors
    ///
    /// If the socket cannot be opened, the handshake times out, the daemon
    /// rejects the hello, or the first records cannot be written.
    pub(crate) fn connect(path: &Path) -> io::Result<Self> {
        let mut stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(HANDSHAKE))?;
        stream.set_write_timeout(Some(HANDSHAKE))?;
        write_record(
            &mut stream,
            &ClientMessage::Hello(Hello::current()).encode(),
        )?;
        let payload = read_record(&mut stream)?;
        match ServerMessage::decode(&payload) {
            Ok(ServerMessage::Welcome { .. }) => {}
            Ok(ServerMessage::Reject(reason)) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("daemon rejected hello: {reason:?}"),
                ));
            }
            Ok(other) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("expected Welcome, got {other:?}"),
                ));
            }
            Err(error) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, error));
            }
        }
        write_record(
            &mut stream,
            &ClientMessage::Subscribe { drive_states: true }.encode(),
        )?;
        stream.set_read_timeout(None)?;
        stream.set_write_timeout(Some(HANDSHAKE))?;
        Ok(Self {
            stream,
            next_request: 1,
        })
    }

    /// MIDI as [`crate::link::Link::midi`]: unknown statuses are dropped.
    ///
    /// # Errors
    ///
    /// If the record cannot be written.
    pub(crate) fn midi(&mut self, bytes: [u8; 3]) -> io::Result<bool> {
        let Some(message) = ClientMessage::midi(self.next_id(), bytes) else {
            return Ok(false);
        };
        self.send(&message)?;
        Ok(true)
    }

    /// Reset of `siren`.
    ///
    /// # Errors
    ///
    /// If the record cannot be written.
    pub(crate) fn reset(&mut self, siren: SirenId) -> io::Result<()> {
        let request = self.next_id();
        self.send(&ClientMessage::reset(request, siren))
    }

    /// Reset of every siren.
    ///
    /// # Errors
    ///
    /// If the record cannot be written.
    pub(crate) fn reset_all(&mut self) -> io::Result<()> {
        let request = self.next_id();
        self.send(&ClientMessage::reset_all(request))
    }

    /// ST of every siren.
    ///
    /// # Errors
    ///
    /// If the record cannot be written.
    pub(crate) fn st_all(&mut self, enabled: bool) -> io::Result<()> {
        let request = self.next_id();
        self.send(&ClientMessage::st_all(request, enabled))
    }

    /// Drive-state records waiting on the socket.
    ///
    /// # Errors
    ///
    /// If the daemon closed the stream, or a record is not a server message.
    pub(crate) fn poll(&mut self) -> io::Result<Vec<ServerMessage>> {
        let mut out = Vec::new();
        loop {
            match self.try_read_record()? {
                None => break,
                Some(payload) => {
                    let message = ServerMessage::decode(&payload)
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                    out.push(message);
                }
            }
        }
        Ok(out)
    }

    fn send(&mut self, message: &ClientMessage) -> io::Result<()> {
        self.stream.set_nonblocking(false)?;
        write_record(&mut self.stream, &message.encode())
    }

    fn try_read_record(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.stream.set_nonblocking(true)?;
        let mut header = [0u8; 4];
        let n = match self.stream.read(&mut header) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "daemon closed",
                ));
            }
            Ok(n) => n,
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.kind() == io::ErrorKind::Interrupted =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        self.stream.set_nonblocking(false)?;
        if n < 4 {
            self.stream.read_exact(&mut header[n..])?;
        }
        let len = usize::try_from(u32::from_le_bytes(header)).unwrap_or(usize::MAX);
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "payload larger than MAX_PAYLOAD",
            ));
        }
        let mut payload = vec![0; len];
        self.stream.read_exact(&mut payload)?;
        Ok(Some(payload))
    }

    fn next_id(&mut self) -> RequestId {
        let id = RequestId(self.next_request);
        self.next_request = self.next_request.saturating_add(1);
        id
    }
}
