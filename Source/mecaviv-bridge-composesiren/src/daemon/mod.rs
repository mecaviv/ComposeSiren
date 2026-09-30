//! Client of `mecaviv-bridge-daemon`.
//!
//! On Unix the socket is [`mecaviv_bridge_daemon::socket_path`]. Windows has
//! no local-socket path in this protocol yet, so [`Client::connect`] fails and
//! the bridge stays in-process.

use std::path::Path;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::Client;

#[cfg(not(unix))]
mod stub {
    use std::io;
    use std::path::Path;

    use mecaviv_bridge_daemon::ServerMessage;
    use mecaviv_v1::SirenId;

    pub(crate) struct Client {
        _private: (),
    }

    impl Client {
        pub(crate) fn connect(_path: &Path) -> io::Result<Self> {
            Err(unsupported())
        }

        pub(crate) fn midi(&mut self, _bytes: [u8; 3]) -> io::Result<bool> {
            Err(unsupported())
        }

        pub(crate) fn reset(&mut self, _siren: SirenId) -> io::Result<()> {
            Err(unsupported())
        }

        pub(crate) fn reset_all(&mut self) -> io::Result<()> {
            Err(unsupported())
        }

        pub(crate) fn st_all(&mut self, _enabled: bool) -> io::Result<()> {
            Err(unsupported())
        }

        pub(crate) fn poll(&mut self) -> io::Result<Vec<ServerMessage>> {
            Err(unsupported())
        }
    }

    fn unsupported() -> io::Error {
        io::Error::new(io::ErrorKind::Unsupported, "the daemon socket is Unix-only")
    }
}

#[cfg(not(unix))]
pub(crate) use stub::Client;

/// Whether `path` accepts a connection. Used for the tooltip while the bridge
/// is disabled: a successful connect is dropped without a hello.
#[must_use]
pub(crate) fn available(path: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::net::UnixStream::connect(path).is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}
