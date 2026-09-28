use std::net::SocketAddr;

use mecaviv_v1::keb::KebModel;
use mecaviv_v1::{Device, SirenId, park};

/// Where a siren and its drive are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SirenEndpoints {
    /// The siren's card, which receives commands.
    pub card: SocketAddr,
    /// The siren's KEB drive, which answers state reads.
    pub drive: SocketAddr,
    /// The drive's model.
    pub model: KebModel,
}

/// Where every siren is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParkTable {
    /// S1 first.
    pub sirens: [SirenEndpoints; 7],
}

impl ParkTable {
    /// The park's default addresses (`mecaviv_v1::park`): cards at
    /// `192.168.1.11..17:8001`, drives at `192.168.1.70..76:8000`, S4 on a
    /// KEB F6.
    ///
    /// # Panics
    ///
    /// Never: every siren has a default card, drive and model.
    #[must_use]
    pub fn defaults() -> Self {
        let endpoints = |siren: SirenId| {
            let device = Device::Siren(siren);
            SirenEndpoints {
                card: park::control_address(device)
                    .expect("sirens have a card")
                    .into(),
                drive: park::drive_address(device)
                    .expect("sirens have a drive")
                    .into(),
                model: park::drive_model(device).expect("sirens have a drive model"),
            }
        };
        Self {
            sirens: std::array::from_fn(|i| {
                endpoints(SirenId::new(u8::try_from(i + 1).expect("7 sirens")).expect("1..=7"))
            }),
        }
    }

    /// The endpoints of `siren`.
    #[must_use]
    pub fn siren(&self, siren: SirenId) -> &SirenEndpoints {
        &self.sirens[usize::from(siren.get() - 1)]
    }

    /// The siren whose drive is at `address`.
    #[must_use]
    pub fn siren_with_drive(&self, address: SocketAddr) -> Option<SirenId> {
        SirenId::all().find(|&siren| self.siren(siren).drive == address)
    }
}

impl Default for ParkTable {
    fn default() -> Self {
        Self::defaults()
    }
}
