//! `frames.skip_silent` also realigns frames whose partial slots don't follow the note's
//! (data.rs, `repair_note`): many S7 notes (and a few of S1, S3, S4, S5) lose a partial at the
//! loop's end, every slot above it moves down, and each slot's oscillator jumps there and back
//! at the wrap, once per loop: a click every 15 to 400 ms.

use siren_dsp::Library;
use siren_dsp::data::slot_moved_share;
use siren_dsp::settings::Engine;
use siren_dsp::siren::SirenId;
use siren_dsp::variants::VARIANTS;

#[path = "click_hp.rs"]
#[allow(dead_code)]
mod click_hp;

fn worst_jump(id: SirenId, n: usize, skip: bool) -> f64 {
    let lib = Library::new(siren_dsp::default_resources());
    let d = lib.data(id.def()).unwrap();
    let i = &d.info[n];
    (0..i.fmax).map(|f| {
        let (a, b) = (d.play_frame(n, f, skip), d.play_frame(n, if f + 1 >= i.fmax { 0 } else { f + 1 }, skip));
        slot_moved_share(d.freq(n, a), d.amp(n, a), d.freq(n, b), d.amp(n, b))
    }).fold(0.0, f64::max)
}

// the notes reported clicking (S7: data note = sounding MIDI - 12) and the S6 one (S5's data)
const NOTES: [(SirenId, usize); 8] = [(SirenId::S7, 44), (SirenId::S7, 47), (SirenId::S7, 50), (SirenId::S7, 52),
    (SirenId::S7, 58), (SirenId::S7, 66), (SirenId::S7, 42), (SirenId::S5, 77)];

#[test]
fn the_slots_follow_the_partials_once_realigned() {
    for (id, n) in NOTES {
        let (off, on) = (worst_jump(id, n, false), worst_jump(id, n, true));
        eprintln!("{id:?} n{n}: worst slot jump {:.0}% -> {:.0}%", off * 100.0, on * 100.0);
        assert!(off > 0.4, "{id:?} n{n} has the defect: {off:.2}");
        assert!(on < 0.35, "{id:?} n{n} realigned: {on:.2}");
    }
}

#[test]
fn s7_f_sharp_5_repairs_the_missing_fundamental_at_the_wrap() {
    let lib = Library::new(siren_dsp::default_resources());
    let data = lib.data(SirenId::S7.def()).unwrap();
    let note = 66;
    let end = data.info[note].fmax - 1;
    let repaired = data.play_frame(note, end, true);
    let before = data.amp(note, end - 1)[0];
    let after = data.amp(note, 0)[0];
    assert!(data.amp(note, repaired)[0] >= before.min(after));
    assert!(data.amp(note, repaired)[0] <= before.max(after));

    // S7 n66: frame 76 of 77 has every partial one slot down: a click every 65 ms
    for var in VARIANTS.iter().filter(|v| v.engine == Engine::Rust && !v.dsp_settings().unwrap().frames_legacy) {
        let (off, _) = click_hp::render(SirenId::S7, var.id, 66, false, 3.0);
        let (on, _) = click_hp::render(SirenId::S7, var.id, 66, true, 3.0);
        let (p_off, p_on) = (click_hp::hp_peaks(&off, 22050)[0].0, click_hp::hp_peaks(&on, 22050)[0].0);
        eprintln!("S7 n66 {:<18} high-passed peak over context: off {p_off:.1} dB, on {p_on:.1} dB", var.id);
        assert!(p_on < 3.0 && p_on < p_off - 0.5, "{}: off {p_off:.1}, on {p_on:.1}", var.id);
    }
}

#[test]
fn s7_trio_fast_smooths_harmonic_changes_from_b_through_d_sharp() {
    let lib = Library::new(siren_dsp::default_resources());
    let mut settings = siren_dsp::settings::variant("trio-fast").unwrap().dsp_settings().unwrap();
    // An exact number of samples per period lets us measure level changes
    // without counting the normal motion of the waveform.
    let period = 48;
    let mut peak_change = |note: i32, repair| {
        let frequency = 440.0 * 2f64.powf((note as f64 - 69.0) / 12.0);
        let rate = frequency * period as f64;
        settings.skip_silent = repair;
        let mut voice = lib.voice(SirenId::S7, rate).unwrap();
        settings.apply(&mut voice);
        voice.glide = false;
        voice.note_on(note + 12, 127);
        let samples: Vec<_> = (0..(rate * 2.0) as usize).map(|_| voice.process()).collect();
        samples[rate as usize..].windows(period + 1)
            .map(|w| (w[period] - w[0]).abs())
            .fold(0.0, f32::max)
    };
    for note in 71..=75 {
        let off = peak_change(note, false);
        let on = peak_change(note, true);
        eprintln!("S7 MIDI {} peak change per cycle: {off:.5} off, {on:.5} on", note + 12);
        assert!(on < off * 0.5, "S7 data note {note}: {off:.5} off, {on:.5} on");
    }
}
