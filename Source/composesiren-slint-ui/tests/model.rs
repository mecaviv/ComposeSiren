//! The parameter model, MIDI mapping and store, without any UI.

use composesiren_slint_ui::midi;
use composesiren_slint_ui::params::{CATEGORIES, PARAMS, ParamDef, ParamId, siren_colour};
use composesiren_slint_ui::store::{ParamStore, param_id};

#[test]
fn table_is_in_id_order_with_sane_bounds() {
    for (i, d) in PARAMS.iter().enumerate() {
        assert_eq!(d.id as usize, i, "{}", d.code_name);
        assert_eq!(param_id(i), Some(d.id));
        assert!(
            d.min < d.max && (d.min..=d.max).contains(&d.default),
            "{}",
            d.code_name
        );
    }
    assert_eq!(param_id(PARAMS.len()), None);
}

#[test]
fn constrain_and_normalise_round_trip() {
    let range = ParamDef::of(ParamId::PitchBendRange);
    assert!((range.constrain(7.4) - 7.0).abs() < f32::EPSILON);
    assert!((range.constrain(99.0) - 36.0).abs() < f32::EPSILON);
    let volume = ParamDef::of(ParamId::Volume);
    for v in [0.0, 1.0, 63.0, 127.0] {
        assert!((volume.denormalise(volume.normalise(v)) - v).abs() < 1e-4);
    }
}

#[test]
fn cc_messages_match_the_plugin() {
    assert_eq!(
        midi::message(ParamDef::of(ParamId::Volume), 100.0, 1),
        Some([0xB0, 7, 100])
    );
    assert_eq!(
        midi::message(ParamDef::of(ParamId::VibratoAmplitude), 64.0, 16),
        Some([0xBF, 1, 64])
    );
    assert_eq!(
        midi::message(ParamDef::of(ParamId::PitchBend), 0.0, 1),
        Some([0xE0, 0x00, 0x40])
    );
    assert_eq!(
        midi::message(ParamDef::of(ParamId::Transpose), 3.0, 1),
        None
    );
}

#[test]
fn store_flags_host_changes_only() {
    let store = ParamStore::default();
    assert!((store.get(ParamId::Volume) - 127.0).abs() < f32::EPSILON);
    store.set_from_ui(ParamId::Timbre, 12.0);
    assert_eq!(store.take_host_changes(), 0);
    store.set_from_host(ParamId::Timbre, 300.0);
    store.set_from_host(ParamId::Mute, 1.0);
    assert_eq!(
        store.take_host_changes(),
        (1 << ParamId::Timbre as u32) | (1 << ParamId::Mute as u32)
    );
    assert!((store.get(ParamId::Timbre) - 127.0).abs() < f32::EPSILON);
    assert_eq!(store.take_host_changes(), 0);
}

#[test]
fn siren_colours_follow_the_blue_ramp() {
    assert_eq!(siren_colour(2), (0x46, 0x50, 0xc8)); // S3 is the first (darkest)
    assert_ne!(siren_colour(CATEGORIES[0].1), siren_colour(CATEGORIES[4].1));
}
