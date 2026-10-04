//! The Slint editor bound to a [`ParamStore`]: rows built from the parameter table, UI changes constrained,
//! stored and handed to the host, host changes polled at frame rate.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::midi;
use crate::params::{CATEGORIES, GROUPS, PARAMS, ParamDef, ParamId, Widget, siren_colour};
use crate::store::{ParamStore, param_id};
use crate::{GroupRow, OneSiren, ParamRow};

/// What the editor tells the host. In the plugin: `beginChangeGesture` / `setValueNotifyingHost` /
/// `endChangeGesture` on the APVTS parameter, and the voice manager's category.
pub trait HostSink {
    /// A parameter moved in the editor (already constrained to its range and step).
    fn param_changed(&self, id: ParamId, value: f32);
    /// A drag or click on a parameter starts (`true`) or ends (`false`).
    fn gesture(&self, id: ParamId, begin: bool);
    /// The siren type menu changed (index into [`CATEGORIES`]).
    fn category_changed(&self, _category: usize) {}
    /// A key of the on-screen keyboard went down or up.
    fn note(&self, _note: u8, _on: bool) {}
}

/// The editor component and its bindings.
pub struct Editor {
    /// The Slint component (show it, run it, or embed its window).
    pub component: OneSiren,
    _poll: Timer,
}

fn row(def: &ParamDef, value: f32) -> ParamRow {
    ParamRow {
        caption: def.caption().into(),
        value,
        text: def.format(value).into(),
        minimum: def.min,
        maximum: def.max,
        step: def.step,
        default_value: def.default,
        kind: match def.widget {
            Widget::Knob => 0,
            Widget::CentredKnob => 1,
            Widget::Spin => 2,
        },
    }
}

/// The parameter rows, one model per group of the strip (see `GroupRow` in `ui/onesiren.slint`).
#[derive(Clone)]
struct Rows(Rc<Vec<Rc<VecModel<ParamRow>>>>);

impl Rows {
    fn new(store: &ParamStore) -> Self {
        Self(Rc::new(
            GROUPS
                .iter()
                .map(|&(_, first, count)| {
                    Rc::new(VecModel::from(
                        PARAMS[first..first + count]
                            .iter()
                            .map(|d| row(d, store.get(d.id)))
                            .collect::<Vec<_>>(),
                    ))
                })
                .collect(),
        ))
    }

    fn groups(&self) -> Vec<GroupRow> {
        GROUPS
            .iter()
            .zip(self.0.iter())
            .map(|(&(title, first, _), rows)| GroupRow {
                title: title.into(),
                first: first as i32,
                params: ModelRc::from(rows.clone()),
            })
            .collect()
    }

    /// Show `value` for parameter `index` (of the parameter table).
    fn set(&self, index: usize, value: f32) {
        let Some((g, &(_, first, _))) = GROUPS
            .iter()
            .enumerate()
            .find(|(_, (_, first, count))| (*first..first + count).contains(&index))
        else {
            return;
        };
        let rows = &self.0[g];
        if let Some(mut r) = rows.row_data(index - first)
            && ((r.value - value).abs() > f32::EPSILON || r.text.is_empty())
        {
            r.value = value;
            r.text = PARAMS[index].format(value).into();
            rows.set_row_data(index - first, r);
        }
    }
}

fn colour(siren: usize) -> slint::Color {
    let (r, g, b) = siren_colour(siren);
    slint::Color::from_rgb_u8(r, g, b)
}

/// Show a siren type chosen elsewhere (the plugin's state, the other editor) without reporting it back.
pub fn show_category(component: &OneSiren, category: usize) {
    let category = category.min(CATEGORIES.len() - 1);
    component.set_category(category as i32);
    component.set_strip_colour(colour(CATEGORIES[category].1));
}

impl Editor {
    /// Build the component for `store`, sending UI changes to `sink`.
    ///
    /// # Errors
    /// When Slint has no platform (no windowing backend, and no embedded window prepared).
    pub fn new(
        store: &Arc<ParamStore>,
        sink: &Rc<dyn HostSink>,
    ) -> Result<Self, slint::PlatformError> {
        let component = OneSiren::new()?;
        let rows = Rows::new(store);
        component.set_groups(ModelRc::new(VecModel::from(rows.groups())));
        component.set_categories(ModelRc::new(VecModel::from(
            CATEGORIES
                .iter()
                .map(|(n, _)| SharedString::from(*n))
                .collect::<Vec<_>>(),
        )));
        show_category(&component, 3); // Soprano until the host says otherwise

        wire_params(&component, store, sink, &rows);
        wire_strip(&component, store, sink, &rows);
        let poll = poll_host_changes(store, &rows);

        Ok(Self {
            component,
            _poll: poll,
        })
    }
}

fn midi_channel(c: &OneSiren) -> u8 {
    if c.get_midi_in() == 0 {
        1
    } else {
        c.get_midi_in() as u8
    }
}

/// Parameter edits: constrain and store, refresh the row, tell the host, show the mirrored MIDI message.
fn wire_params(
    component: &OneSiren,
    store: &Arc<ParamStore>,
    sink: &Rc<dyn HostSink>,
    rows: &Rows,
) {
    component.on_param_changed({
        let (store, sink, rows, weak) = (
            store.clone(),
            sink.clone(),
            rows.clone(),
            component.as_weak(),
        );
        move |index, value| {
            let Some(id) = param_id(index as usize) else {
                return;
            };
            let before = store.get(id);
            let stored = store.set_from_ui(id, value);
            rows.set(index as usize, stored);
            if (stored - before).abs() > f32::EPSILON {
                sink.param_changed(id, stored);
                if let Some(c) = weak.upgrade()
                    && let Some(m) = midi::message(ParamDef::of(id), stored, midi_channel(&c))
                {
                    c.set_midi_out_text(format!("MIDI out: {}", midi::describe(m)).into());
                }
            }
        }
    });
    component.on_param_gesture({
        let sink = sink.clone();
        move |index, begin| {
            if let Some(id) = param_id(index as usize) {
                sink.gesture(id, begin);
            }
        }
    });
}

/// Header and keyboard: siren type, reset, notes.
fn wire_strip(component: &OneSiren, store: &Arc<ParamStore>, sink: &Rc<dyn HostSink>, rows: &Rows) {
    component.on_category_picked({
        let (sink, weak) = (sink.clone(), component.as_weak());
        move |c| {
            let c = (c.max(0) as usize).min(CATEGORIES.len() - 1);
            if let Some(ui) = weak.upgrade() {
                ui.set_strip_colour(colour(CATEGORIES[c].1));
            }
            sink.category_changed(c);
        }
    });
    component.on_reset_all({
        let (store, sink, rows) = (store.clone(), sink.clone(), rows.clone());
        move || {
            for (i, d) in PARAMS.iter().enumerate() {
                sink.gesture(d.id, true);
                let v = store.set_from_ui(d.id, d.default);
                sink.param_changed(d.id, v);
                sink.gesture(d.id, false);
                rows.set(i, v);
            }
        }
    });
    component.on_note({
        let (sink, weak) = (sink.clone(), component.as_weak());
        move |note, on| {
            let note = note.clamp(0, 127) as u8;
            sink.note(note, on);
            if let Some(c) = weak.upgrade() {
                let ch = midi_channel(&c) - 1;
                let m = if on {
                    [0x90 | ch, note, 100]
                } else {
                    [0x80 | ch, note, 0]
                };
                c.set_midi_out_text(format!("MIDI out: {}", midi::describe(m)).into());
            }
        }
    });
}

/// Host -> editor: automation, presets, MIDI input change values on other threads; the editor looks at
/// the change mask at frame rate and refreshes only the rows that moved.
fn poll_host_changes(store: &Arc<ParamStore>, rows: &Rows) -> Timer {
    let poll = Timer::default();
    poll.start(TimerMode::Repeated, Duration::from_millis(16), {
        let (store, rows) = (store.clone(), rows.clone());
        move || {
            let mut mask = store.take_host_changes();
            while mask != 0 {
                let i = mask.trailing_zeros() as usize;
                mask &= mask - 1;
                rows.set(i, store.get(PARAMS[i].id));
            }
        }
    });
    poll
}
