//! ComposeSiren's click output: the click box's engine (`clic` from
//! firmwares-artila's `ClicRaspberry/rs`), driven by MIDI, rendered to a
//! stereo bus.
//!
//! The MIDI rule is the box's: channel 10, note on with note > 1 and
//! velocity > 1, an even note plays the strong click, an odd one the weak
//! click; a program change on channel 10 picks the click (0 = the first).
//!
//! [`Clic::midi`] and [`Clic::render`] neither allocate nor block: they run on
//! the audio thread. [`Clic::set_sample_rate`] reloads the clicks at the new
//! rate, from `prepareToPlay`.

use clic::bank;
use clic::midi::{self, Action};
use clic::mixer::Mixer;

mod ffi;

/// The click engine of one plugin instance.
pub struct Clic {
    mixer: Mixer,
    rate: u32,
}

impl Clic {
    /// The built-in clicks, at `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: f64) -> Clic {
        let rate = to_rate(sample_rate);
        Clic { mixer: Mixer::new(bank::builtin(rate)), rate }
    }

    /// Reloads the clicks at `sample_rate`, keeping the chosen click.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        let rate = to_rate(sample_rate);
        if rate != self.rate {
            let current = self.mixer.current();
            self.mixer = Mixer::new(bank::builtin(rate));
            let _ = self.mixer.set_clic(current);
            self.rate = rate;
        }
    }

    /// A MIDI message; anything but the click's is ignored.
    pub fn midi(&mut self, status: u8, data1: u8, data2: u8) {
        match midi::action(status, data1, data2) {
            Some(Action::Clic(which)) => self.mixer.trigger(which),
            Some(Action::Choix(n)) => {
                let _ = self.mixer.set_clic(n);
            }
            None => {}
        }
    }

    /// Renders the next `min(left.len(), right.len())` frames.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.mixer.render_f32(left, right);
    }

    /// The chosen click.
    #[must_use]
    pub fn current(&self) -> usize {
        self.mixer.current()
    }

    /// How many clicks there are.
    #[must_use]
    pub fn count(&self) -> usize {
        self.mixer.clics().len()
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
