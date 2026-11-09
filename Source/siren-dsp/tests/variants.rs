//! The variants and their settings: every variant parses and plays (the rotor spins up for
//! seconds: "plays" is a low bar), `rust-exact` is the C++ bit
//! for bit, the time scaling alone is exact at 44.1 kHz, and the settings refuse what they don't know.

use siren_dsp::Library;
use siren_dsp::settings::{DspSettings, Engine, variant};
use siren_dsp::siren::SirenId;
use siren_dsp::variants::{DATASETS, SWITCHES, VARIANTS};
use siren_dsp_oracle::CppVoice;

fn render(lib: &Library, id: SirenId, s: &DspSettings, sr: f64, note: i32, samples: usize) -> Vec<f32> {
    let mut v = lib.voice(id, sr).unwrap();
    s.apply(&mut v);
    v.note_on(note, 127);
    (0..samples).map(|_| v.process()).collect()
}

fn lib() -> Library {
    Library::new(siren_dsp::default_resources())
}

#[test]
fn every_variant_parses_and_plays() {
    let lib = lib();
    assert_eq!(VARIANTS[0].id, "cpp", "the default is the C++ DSP");
    for v in VARIANTS {
        let s = v.dsp_settings().unwrap_or_else(|e| panic!("{}: {e}", v.id));
        assert_eq!(s.continuous_vibrato, !matches!(v.id, "cpp" | "rust-exact"), "{}", v.id);
        if v.engine == Engine::Cpp {
            continue;
        }
        for (id, note) in [(SirenId::S1, 72), (SirenId::S7, 88)] {
            let out = render(&lib, id, &s, 48000.0, note, 3 * 48000);
            assert!(out.iter().all(|x| x.is_finite()), "{} {id:?}: not finite", v.id);
            let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 1e-3 && peak < 4.0, "{} {id:?}: peak {peak}", v.id);
        }
    }
    let mut ids: Vec<_> = VARIANTS.iter().map(|v| v.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), VARIANTS.len(), "variant ids are unique");
}

#[test]
fn rust_exact_is_the_cpp_bit_for_bit() {
    let lib = lib();
    let s = variant("rust-exact").unwrap().dsp_settings().unwrap();
    assert_eq!(s, DspSettings::default());
    for (siren, id, note, sr) in [("S1", SirenId::S1, 72, 44100.0), ("S4", SirenId::S4, 70, 48000.0)] {
        let samples = 4 * sr as usize;
        let mut cpp = CppVoice::new(&siren_dsp::default_resources(), siren, sr).unwrap();
        cpp.note_on(note, 127);
        let mut want = vec![0.0f32; samples];
        cpp.render(&mut want);
        let got = render(&lib, id, &s, sr, note, samples);
        let first = got.iter().zip(&want).position(|(a, b)| a.to_bits() != b.to_bits());
        assert_eq!(first, None, "{siren} @{sr}: rust-exact differs from the C++");
    }
}

#[test]
fn time_scaling_is_rust_exact_at_44k_without_vibrato() {
    let lib = lib();
    let exact = render(&lib, SirenId::S3, &DspSettings::default(), 44100.0, 66, 3 * 44100);
    let mut time_settings = variant("rust-time").unwrap().dsp_settings().unwrap();
    time_settings.position_spectrum = false;
    let time = render(&lib, SirenId::S3, &time_settings, 44100.0, 66, 3 * 44100);
    assert!(exact.iter().zip(&time).all(|(a, b)| a.to_bits() == b.to_bits()));
}

#[test]
fn rust_time_spectrum_follows_position_without_advancing_frames() {
    let lib = lib();
    let mut v = lib.voice(SirenId::S7, 48_000.0).unwrap();
    variant("rust-time").unwrap().dsp_settings().unwrap().apply(&mut v);
    v.siren.set_16ou8bit(false);

    v.siren.jump_to(7000);
    v.siren.calculwave();
    let lower = v.siren.handoff().amp;
    v.siren.jump_to(7100);
    v.siren.calculwave();
    let upper = v.siren.handoff().amp;
    assert_ne!(lower, upper);

    v.siren.jump_to(7050);
    v.siren.calculwave();
    let midpoint = v.siren.handoff().amp;
    for ((a, b), m) in lower.iter().zip(upper).zip(midpoint) {
        assert!((m - (a + b) * 0.5).abs() < 1e-6);
    }
    for _ in 0..4800 {
        v.siren.calculwave();
    }
    assert_eq!(v.siren.handoff().amp, midpoint);

    v.siren.jump_to(7000);
    v.siren.calculwave();
    v.siren.jump_to(7050);
    v.siren.calculwave();
    assert_eq!(v.siren.handoff().amp, midpoint);
}

#[test]
fn continuous_vibrato_leaves_the_rotor_at_the_held_note() {
    let lib = lib();
    for id in ["rust-time", "frames", "trio", "trio-fast", "trio-f32"] {
        let settings = variant(id).unwrap().dsp_settings().unwrap();
        let mut dry = lib.voice(SirenId::S7, 48_000.0).unwrap();
        let mut wet = lib.voice(SirenId::S7, 48_000.0).unwrap();
        settings.apply(&mut dry);
        settings.apply(&mut wet);
        wet.control_change(9, 108);
        wet.control_change(1, 3);
        dry.note_on(83, 100);
        wet.note_on(83, 100);

        let mut different = false;
        for _ in 0..3 * 48_000 {
            let (a, b) = (dry.process(), wet.process());
            assert_eq!(dry.siren.midicent(), wet.siren.midicent(), "{id}");
            assert!(b.is_finite(), "{id}");
            different |= a.to_bits() != b.to_bits();
        }
        assert!(different, "{id}: oscillator vibrato must change the sound");
    }
}

#[test]
fn settings_refuse_what_they_do_not_know() {
    let mut s = DspSettings::default();
    assert!(s.set("trio.kernel", "cubic").is_err());
    assert!(s.set("frames.speed", "9").is_err());
    assert!(s.set("synth.glide", "maybe").is_err());
    assert!(s.set("no.such", "1").is_err());
    for key in DspSettings::KEYS {
        // every listed key is known (the value may be refused, not the key)
        let e = s.set(key, "?").unwrap_err();
        assert_ne!(e.reason, "unknown setting", "{key}");
    }
    s.set("trio.kernel", "nearest").unwrap();
    assert!(s.trio.is_some());
    s.set("trio.kernel", "off").unwrap();
    assert!(s.trio.is_none());
}

/// The generated tables agree with this crate: the metadata's switches are exactly the keys
/// `DspSettings` knows, each default parses, and the default data set is the fixed one.
#[test]
fn the_metadata_matches_the_settings() {
    let ids: Vec<&str> = SWITCHES.iter().map(|s| s.id).collect();
    assert_eq!(ids, DspSettings::KEYS, "siren-dsp-switches.csv against DspSettings::KEYS");
    let mut s = DspSettings::default();
    for w in SWITCHES {
        s.set(w.id, w.default).unwrap_or_else(|e| panic!("{e}"));
    }
    assert_eq!(DATASETS[0].id, "fixed");
    assert!(siren_dsp::settings::dataset("raw").is_some());
}
