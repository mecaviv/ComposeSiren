//! `SirenOrchestra`'s editor: seven tracks, reverb, master and (with `COMPOSESIREN_CLIC`) the clic, built
//! from the generated metadata (`metadata::PARAMETERS`, `SECTIONS`, `SIRENS`): the same tables the JUCE
//! editor's `UiMetadata.h` holds.
//!
//! The parameter table lists every parameter the editor shows as `(group, parameter)`, the JUCE id being
//! `"<group> | <code name>"`: per track (S7 on top .. S3 at the bottom, `sirenOrder`) its 14 siren
//! parameters by section and position, then `TrackPanning` and `TrackOutputGain`; then the reverb (`R`),
//! the master volume (`M`) and the clic (`C`). Values live in an [`OrchestraStore`] (lock-free, like
//! [`crate::store::ParamStore`]); the host pushes values in, the editor polls them at frame rate.

use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::{ComponentHandle, Model, ModelRc, PhysicalSize, Timer, TimerMode, VecModel};

use crate::embed::{self, Bgra8Premultiplied, DirtyRect, Pointer};
use crate::metadata::{self, Midi, Parameter, ParameterClass, Widget};
use crate::{GroupRow, ParamRow, SirenOrchestra, TrackRow};

/// The tracks from top to bottom (`sirenOrder` in the JUCE editor): siren id, title, MIDI channel.
pub const TRACKS: [(&str, &str, u8); 7] = [
    ("S7", "Piccolo", 7),
    ("S6", "Soprano 2", 6),
    ("S5", "Soprano 1", 5),
    ("S2", "Alto 2", 2),
    ("S1", "Alto 1", 1),
    ("S4", "Tenor", 4),
    ("S3", "Bass", 3),
];

/// The editor's size in logical pixels (the JUCE editor's).
pub const LOGICAL_SIZE: (f32, f32) = (928.0, 630.0);

/// One parameter of the editor.
#[derive(Clone, Copy, Debug)]
pub struct OrchParam {
    /// `S1`..`S7`, `R`, `M` or `C`.
    pub group: &'static str,
    /// Its metadata.
    pub meta: &'static Parameter,
}

impl OrchParam {
    /// The JUCE parameter id.
    #[must_use]
    pub fn juce_id(&self) -> String {
        format!("{} | {}", self.group, self.meta.code_name)
    }

    /// Clamp to the range and snap to the step.
    #[must_use]
    pub fn constrain(&self, v: f32) -> f32 {
        let m = self.meta;
        let v = v.clamp(m.min, m.max);
        if m.step > 0.0 {
            (((v - m.min) / m.step).round() * m.step + m.min).clamp(m.min, m.max)
        } else {
            v
        }
    }

    /// The text under a knob: integers for stepped parameters, two decimals otherwise.
    #[must_use]
    pub fn format(&self, v: f32) -> String {
        if self.meta.step >= 1.0 {
            format!("{}", v.round() as i32)
        } else if self.meta.max > 100.0 {
            format!("{v:.0}")
        } else {
            format!("{v:.2}")
        }
    }

    fn caption(&self) -> String {
        let label = match self.meta.code_name {
            "TrackPanning" => "Pan",
            "TrackOutputGain" => "Gain",
            "ClicSpread" => "Spread",
            "ClicDecay" => "Decay",
            "ClicBias" => "Bias",
            "ClicVolume" => "Vol",
            _ => self.meta.strip_label,
        };
        match (self.meta.class, self.meta.midi) {
            (ParameterClass::Siren, Midi::Cc(cc)) => format!("{label}\ncc{cc}"),
            _ => label.to_string(),
        }
    }

    fn kind(&self) -> i32 {
        match (self.meta.code_name, self.meta.widget) {
            ("MasterVolume" | "ClicVolume", _) => 4, // NotchedKnobLAF
            (_, Widget::CentredKnob) => 1,
            (_, Widget::Spin) => 2,
            (_, Widget::Toggle) => 3,
            _ => 0,
        }
    }

    fn row(&self, value: f32) -> ParamRow {
        ParamRow {
            caption: self.caption().into(),
            value,
            text: self.format(value).into(),
            minimum: self.meta.min,
            maximum: self.meta.max,
            step: self.meta.step,
            default_value: self.meta.default,
            kind: self.kind(),
        }
    }
}

fn param(code_name: &str) -> &'static Parameter {
    metadata::PARAMETERS
        .iter()
        .find(|p| p.code_name == code_name)
        .unwrap_or_else(|| panic!("{code_name} is in the metadata"))
}

/// The siren parameters of a strip, by section and position.
fn siren_params() -> Vec<&'static Parameter> {
    let mut v: Vec<_> = metadata::PARAMETERS
        .iter()
        .filter(|p| p.class == ParameterClass::Siren && p.section.is_some())
        .collect();
    v.sort_by_key(|p| p.section);
    v
}

/// The editor's parameters, in table order (see the module documentation).
#[must_use]
pub fn table(clic: bool) -> Vec<OrchParam> {
    let mut t = Vec::new();
    for (sid, _, _) in TRACKS {
        for meta in siren_params() {
            t.push(OrchParam { group: sid, meta });
        }
        for code in ["TrackPanning", "TrackOutputGain"] {
            t.push(OrchParam { group: sid, meta: param(code) });
        }
    }
    let mut reverb: Vec<_> = metadata::PARAMETERS
        .iter()
        .filter(|p| p.class == ParameterClass::Reverb)
        .collect();
    reverb.sort_by_key(|p| p.section);
    for meta in reverb {
        t.push(OrchParam { group: "R", meta });
    }
    t.push(OrchParam { group: "M", meta: param("MasterVolume") });
    if clic {
        for code in ["ClicEnable", "ClicSpread", "ClicDecay", "ClicBias", "ClicVolume"] {
            t.push(OrchParam { group: "C", meta: param(code) });
        }
    }
    t
}

/// The values of the editor's parameters, shared by the host side and the editor (lock-free).
pub struct OrchestraStore {
    params: Vec<OrchParam>,
    values: Vec<AtomicU32>,
    changed: Vec<AtomicBool>,
    any_change: AtomicBool,
}

impl OrchestraStore {
    /// The store of [`table`]`(clic)`, at the defaults.
    #[must_use]
    pub fn new(clic: bool) -> Self {
        let params = table(clic);
        let values = params.iter().map(|p| AtomicU32::new(p.meta.default.to_bits())).collect();
        let changed = params.iter().map(|_| AtomicBool::new(false)).collect();
        Self { params, values, changed, any_change: AtomicBool::new(false) }
    }

    /// The parameter table.
    #[must_use]
    pub fn params(&self) -> &[OrchParam] {
        &self.params
    }

    /// The current value of parameter `i`.
    #[must_use]
    pub fn get(&self, i: usize) -> f32 {
        self.values.get(i).map_or(0.0, |v| f32::from_bits(v.load(Ordering::Relaxed)))
    }

    /// A value set by the host: constrained, stored and flagged for the editor. Any thread.
    pub fn set_from_host(&self, i: usize, v: f32) {
        if let Some(p) = self.params.get(i) {
            self.values[i].store(p.constrain(v).to_bits(), Ordering::Relaxed);
            self.changed[i].store(true, Ordering::Release);
            self.any_change.store(true, Ordering::Release);
        }
    }

    /// A value set in the editor: constrained and stored, not flagged. Returns the stored value.
    pub fn set_from_ui(&self, i: usize, v: f32) -> Option<f32> {
        let p = self.params.get(i)?;
        let v = p.constrain(v);
        self.values[i].store(v.to_bits(), Ordering::Relaxed);
        Some(v)
    }

    fn take_changes(&self) -> Vec<usize> {
        if !self.any_change.swap(false, Ordering::Acquire) {
            return Vec::new();
        }
        (0..self.params.len())
            .filter(|&i| self.changed[i].swap(false, Ordering::Acquire))
            .collect()
    }
}

/// What the orchestra editor tells the host.
pub trait OrchestraSink {
    /// A parameter moved (index into the table, value in the parameter's own range).
    fn param_changed(&self, index: usize, value: f32);
    /// A gesture on a parameter begins or ends.
    fn gesture(&self, index: usize, begin: bool);
    /// A track's title was clicked (0 = the top track, `TRACKS` order).
    fn track_selected(&self, _track: usize) {}
    /// Reset (the selected siren) or Reset All.
    fn reset(&self, _all: bool) {}
    /// The Menu button.
    fn menu(&self) {}
    /// "Sirenes physiques" or "ST" toggled (park bridge builds).
    fn park_switch(&self, _st: bool, _on: bool) {}
    /// A key of the on-screen keyboard went down or up.
    fn note(&self, _note: u8, _on: bool) {}
}

// Where a parameter is shown: (model, row).
#[derive(Clone)]
struct Models {
    rows: Rc<Vec<Rc<VecModel<ParamRow>>>>,
    place: Rc<Vec<(usize, usize)>>,
}

impl Models {
    fn set(&self, store: &OrchestraStore, i: usize) {
        let Some(&(m, r)) = self.place.get(i) else { return };
        let v = store.get(i);
        let model = &self.rows[m];
        if let Some(mut row) = model.row_data(r)
            && (row.value - v).abs() > f32::EPSILON
        {
            row.value = v;
            row.text = store.params()[i].format(v).into();
            model.set_row_data(r, row);
        }
    }
}

fn colour(rgb: u32) -> slint::Color {
    slint::Color::from_argb_encoded(rgb)
}

fn siren_colour(sid: &str) -> slint::Color {
    metadata::SIRENS.iter().find(|s| s.id == sid).map_or(slint::Color::from_rgb_u8(0x46, 0x50, 0xc8), |s| colour(s.strip_colour))
}

/// The orchestra component bound to a store.
pub struct OrchestraEditor {
    /// The Slint component.
    pub component: SirenOrchestra,
    tracks: Rc<VecModel<TrackRow>>,
    _poll: Timer,
}

impl OrchestraEditor {
    /// Build the component for `store`, sending UI changes to `sink`. `park_bridge`: the build has
    /// `COMPOSESIREN_PARK_BRIDGE` (Sirenes physiques, ST, ST LEDs).
    ///
    /// # Errors
    /// When Slint has no platform.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        store: &Arc<OrchestraStore>,
        sink: &Rc<dyn OrchestraSink>,
        park_bridge: bool,
    ) -> Result<Self, slint::PlatformError> {
        let ui = SirenOrchestra::new()?;
        let params = store.params();
        let mut rows: Vec<Rc<VecModel<ParamRow>>> = Vec::new();
        let mut place = vec![(0, 0); params.len()];
        let mut add_model = |indices: &[usize]| -> (Rc<VecModel<ParamRow>>, i32) {
            let m = rows.len();
            for (r, &i) in indices.iter().enumerate() {
                place[i] = (m, r);
            }
            let model = Rc::new(VecModel::from(
                indices.iter().map(|&i| params[i].row(store.get(i))).collect::<Vec<_>>(),
            ));
            rows.push(model.clone());
            (model, indices.first().map_or(0, |&i| i as i32))
        };

        // tracks: the siren sections, then "Master" (Pan, Gain)
        let per_track = siren_params().len() + 2;
        let mut track_rows = Vec::new();
        for (t, (sid, title, ch)) in TRACKS.iter().enumerate() {
            let base = t * per_track;
            let mut groups = Vec::new();
            for (s, section) in metadata::SECTIONS.iter().enumerate().filter(|(_, s)| s.class == ParameterClass::Siren) {
                let idx: Vec<usize> = (base..base + per_track - 2)
                    .filter(|&i| params[i].meta.section.map(|(g, _)| g) == Some(s))
                    .collect();
                let (model, first) = add_model(&idx);
                groups.push(GroupRow { title: section.title.into(), first, params: ModelRc::from(model) });
            }
            let (model, first) = add_model(&[base + per_track - 2, base + per_track - 1]);
            groups.push(GroupRow { title: "Master".into(), first, params: ModelRc::from(model) });
            track_rows.push(TrackRow {
                title: format!("{title}\nch{ch}").into(),
                colour: siren_colour(sid),
                selected: t == 4, // Alto 1, channel 1, until the host says otherwise
                playing: false,
                groups: ModelRc::new(VecModel::from(groups)),
            });
        }
        let tracks = Rc::new(VecModel::from(track_rows));
        ui.set_tracks(ModelRc::from(tracks.clone()));
        ui.set_bottom_colour(siren_colour(TRACKS[6].0));
        ui.set_park_bridge(park_bridge);

        // reverb: Enable, then the reverb and filter sections
        let find = |group: &str, code: &str| params.iter().position(|p| p.group == group && p.meta.code_name == code);
        if let Some(e) = find("R", "ReverbEnable") {
            let (model, first) = add_model(&[e]);
            ui.set_reverb_enable_index(first);
            ui.set_reverb_enable(model.row_data(0).unwrap_or_default());
        }
        let mut reverb_groups = Vec::new();
        for (s, section) in metadata::SECTIONS.iter().enumerate().filter(|(_, s)| s.class == ParameterClass::Reverb && s.id != "reverb_enable") {
            let idx: Vec<usize> = (0..params.len())
                .filter(|&i| params[i].group == "R" && params[i].meta.section.map(|(g, _)| g) == Some(s))
                .collect();
            let (model, first) = add_model(&idx);
            reverb_groups.push(GroupRow { title: section.title.into(), first, params: ModelRc::from(model) });
        }
        ui.set_reverb_groups(ModelRc::new(VecModel::from(reverb_groups)));
        if let Some(m) = find("M", "MasterVolume") {
            let (model, first) = add_model(&[m]);
            ui.set_master_index(first);
            ui.set_master(model.row_data(0).unwrap_or_default());
        }
        if let Some(c) = find("C", "ClicEnable") {
            let (model, first) = add_model(&[c, c + 1, c + 2, c + 3, c + 4]);
            ui.set_has_clic(true);
            ui.set_clic_first(first);
            ui.set_clic(ModelRc::from(model));
        }

        let models = Models { rows: Rc::new(rows), place: Rc::new(place) };
        wire(&ui, store, sink, &models, &tracks);
        let poll = Timer::default();
        poll.start(TimerMode::Repeated, Duration::from_millis(16), {
            let (store, models, weak) = (store.clone(), models.clone(), ui.as_weak());
            move || {
                for i in store.take_changes() {
                    models.set(&store, i);
                }
                if let Some(ui) = weak.upgrade() {
                    refresh_singles(&ui, &store);
                }
            }
        });
        Ok(Self { component: ui, tracks, _poll: poll })
    }

    /// Select a track (0 = top) without reporting it back, as the host's MIDI input channel says.
    pub fn select_track(&self, track: usize) {
        for t in 0..self.tracks.row_count() {
            if let Some(mut row) = self.tracks.row_data(t)
                && row.selected != (t == track)
            {
                row.selected = t == track;
                self.tracks.set_row_data(t, row);
            }
        }
    }

    /// Light a track's note LED.
    pub fn set_playing(&self, track: usize, playing: bool) {
        if let Some(mut row) = self.tracks.row_data(track)
            && row.playing != playing
        {
            row.playing = playing;
            self.tracks.set_row_data(track, row);
        }
    }
}

// The single-row properties (reverb enable, master) are plain struct properties, not models: refresh them.
fn refresh_singles(ui: &SirenOrchestra, store: &OrchestraStore) {
    for (index, get, set) in [
        (ui.get_reverb_enable_index(), SirenOrchestra::get_reverb_enable as fn(&SirenOrchestra) -> ParamRow, SirenOrchestra::set_reverb_enable as fn(&SirenOrchestra, ParamRow)),
        (ui.get_master_index(), SirenOrchestra::get_master, SirenOrchestra::set_master),
    ] {
        let i = index as usize;
        if i >= store.params().len() {
            continue;
        }
        let v = store.get(i);
        let mut row = get(ui);
        if (row.value - v).abs() > f32::EPSILON {
            row.value = v;
            row.text = store.params()[i].format(v).into();
            set(ui, row);
        }
    }
}

fn wire(
    ui: &SirenOrchestra,
    store: &Arc<OrchestraStore>,
    sink: &Rc<dyn OrchestraSink>,
    models: &Models,
    tracks: &Rc<VecModel<TrackRow>>,
) {
    ui.on_param_changed({
        let (store, sink, models, weak) = (store.clone(), sink.clone(), models.clone(), ui.as_weak());
        move |index, value| {
            let i = index as usize;
            let before = store.get(i);
            let Some(stored) = store.set_from_ui(i, value) else { return };
            models.set(&store, i);
            if let Some(ui) = weak.upgrade() {
                refresh_singles(&ui, &store);
            }
            if (stored - before).abs() > f32::EPSILON {
                sink.param_changed(i, stored);
            }
        }
    });
    ui.on_param_gesture({
        let sink = sink.clone();
        move |index, begin| sink.gesture(index as usize, begin)
    });
    ui.on_track_clicked({
        let (sink, tracks) = (sink.clone(), tracks.clone());
        move |t| {
            let t = t.max(0) as usize;
            for k in 0..tracks.row_count() {
                if let Some(mut row) = tracks.row_data(k)
                    && row.selected != (k == t)
                {
                    row.selected = k == t;
                    tracks.set_row_data(k, row);
                }
            }
            sink.track_selected(t);
        }
    });
    ui.on_reset({
        let sink = sink.clone();
        move || sink.reset(false)
    });
    ui.on_reset_all({
        let sink = sink.clone();
        move || sink.reset(true)
    });
    ui.on_menu({
        let sink = sink.clone();
        move || sink.menu()
    });
    ui.on_physical_sirens_toggled({
        let sink = sink.clone();
        move |on| sink.park_switch(false, on)
    });
    ui.on_st_all_toggled({
        let sink = sink.clone();
        move |on| sink.park_switch(true, on)
    });
    ui.on_note({
        let sink = sink.clone();
        move |n, on| sink.note(n.clamp(0, 127) as u8, on)
    });
}

/// The orchestra editor rendered into host pixels (see [`crate::embed`]).
pub struct EmbeddedOrchestra {
    window: Rc<MinimalSoftwareWindow>,
    editor: OrchestraEditor,
    size: PhysicalSize,
}

impl EmbeddedOrchestra {
    /// The editor at `scale` physical pixels per logical pixel, drawing into pixels the host keeps.
    ///
    /// # Errors
    /// When the component cannot be created.
    pub fn new(
        store: &Arc<OrchestraStore>,
        sink: &Rc<dyn OrchestraSink>,
        park_bridge: bool,
        scale: f32,
    ) -> Result<Self, slint::PlatformError> {
        let window = embed::prepare_window(RepaintBufferType::ReusedBuffer);
        let editor = OrchestraEditor::new(store, sink, park_bridge)?;
        let size = embed::size_window(&window, LOGICAL_SIZE, scale);
        editor.component.show()?;
        Ok(Self { window, editor, size })
    }

    /// The editor.
    #[must_use]
    pub fn editor(&self) -> &OrchestraEditor {
        &self.editor
    }

    /// Width and height in physical pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.size.width, self.size.height)
    }

    /// Run queued work and redraw what changed; the redrawn rectangle.
    pub fn tick_region(&self, pixels: &mut [Bgra8Premultiplied], stride: usize) -> Option<DirtyRect> {
        embed::tick_window(&self.window, pixels, stride)
    }

    /// Forward a pointer event at logical coordinates.
    pub fn pointer(&self, kind: Pointer, x: f32, y: f32) {
        embed::pointer_window(&self.window, kind, x, y);
    }

    /// Forward a wheel event.
    pub fn wheel(&self, x: f32, y: f32, dx: f32, dy: f32) {
        embed::wheel_window(&self.window, x, y, dx, dy);
    }
}
