//! ComposeSiren's click output: the click box's engine (`clic-core` from
//! firmwares-artila's `ClicRaspberry/clic-core`), driven by MIDI, rendered to a
//! stereo bus.
//!
//! The MIDI rule is the box's: channel 10, note on with note > 1 and
//! velocity > 1, an even note plays the strong click, an odd one the weak
//! click; a program change on channel 10 picks the click (0 = the first).
//!
//! [`Clic::midi`] and [`Clic::render`] neither allocate nor block: they run on
//! the audio thread. [`Clic::set_sample_rate`] reloads the clicks at the new
//! rate, from `prepareToPlay`.

use clic_core::{Accent, Bank, Player};

mod ffi;

/// Frames rendered per pass through the interleaved stack buffer.
const CHUNK: usize = 256;

/// The click engine of one plugin instance.
pub struct Clic {
    player: Player,
    rate: u32,
}

impl Clic {
    /// The built-in clicks, at `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: f64) -> Clic {
        let rate = to_rate(sample_rate);
        Clic { player: Player::new(Bank::embedded(), rate), rate }
    }

    /// Reloads the clicks at `sample_rate`, keeping the chosen click.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        let rate = to_rate(sample_rate);
        if rate != self.rate {
            let current = self.player.current();
            self.player = Player::new(Bank::embedded(), rate);
            let _ = self.player.set_clic(current);
            self.rate = rate;
        }
    }

    /// A MIDI message; anything but the click's is ignored.
    pub fn midi(&mut self, status: u8, data1: u8, data2: u8) {
        match (status, data1, data2) {
            // Note on, channel 10, note > 1 and velocity > 1: even = strong.
            (0x99, note, velocity) if note > 1 && velocity > 1 => {
                self.player.trigger(if note % 2 == 0 { Accent::Fort } else { Accent::Faible });
            }
            // Program change, channel 10: an unknown click is ignored.
            (0xc9, program, _) => {
                let _ = self.player.set_clic(usize::from(program));
            }
            _ => {}
        }
    }

    /// Renders the next `min(left.len(), right.len())` frames.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mut block = [0.0f32; 2 * CHUNK];
        for (l, r) in left.chunks_mut(CHUNK).zip(right.chunks_mut(CHUNK)) {
            let n = l.len().min(r.len());
            self.player.render_f32(&mut block[..2 * n]);
            for (i, frame) in block[..2 * n].chunks_exact(2).enumerate() {
                l[i] = frame[0];
                r[i] = frame[1];
            }
        }
    }

/// The chosen click.
    #[must_use]
    pub fn current(&self) -> usize {
        self.player.current()
    }

    /// How many clicks there are.
    #[must_use]
    pub fn count(&self) -> usize {
        Bank::embedded().len()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounded to 1..=u32::MAX just before"
)]
fn to_rate(sample_rate: f64) -> u32 {
    if sample_rate.is_finite() && (1.0..=f64::from(u32::MAX)).contains(&sample_rate) {
        sample_rate.round() as u32
    } else {
        48_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(c: &mut Clic, n: usize) -> Vec<f32> {
        let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
        c.render(&mut l, &mut r);
        l
    }

    #[test]
    fn la_regle_du_boitier() {
        let mut c = Clic::new(48_000.0);
        assert!(render(&mut c, 512).iter().all(|&x| x == 0.0));
        c.midi(0x98, 36, 100); // canal 9 : rien
        assert!(render(&mut c, 512).iter().all(|&x| x == 0.0));
        c.midi(0x99, 36, 100); // canal 10, note paire : clic fort
        assert!(render(&mut c, 512).iter().any(|&x| x != 0.0));
        c.midi(0xc9, 1, 0);
        assert_eq!(c.current(), 1);
        c.midi(0xc9, 9, 0); // clic absent : ignore
        assert_eq!(c.current(), 1);
    }

    #[test]
    fn frequence_de_l_hote() {
        let mut c = Clic::new(44_100.0);
        c.midi(0xc9, 1, 0);
        c.set_sample_rate(96_000.0);
        assert_eq!(c.current(), 1);
        c.midi(0x99, 36, 100);
        let out = render(&mut c, 8000);
        let last = out.iter().rposition(|&x| x != 0.0).unwrap();
        // 70 ms de clave a 96 kHz
        assert!((6700..=6730).contains(&last), "{last}");
    }
}
