//! How long the embedded editor (the plugin path) takes to render one tick, at 1x and 2x, for:
//! a full frame; a knob dragged in the UI (model row + header MIDI text) with full (`NewBuffer`) and
//! partial (`ReusedBuffer`) redraws; the header text alone; and host automation (store → poll → knob).
//! `cargo run --release --example frame_time`

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use composesiren_slint_ui::editor::HostSink;
use composesiren_slint_ui::embed::{Bgra8Premultiplied, EmbeddedEditor};
use composesiren_slint_ui::params::ParamId;
use composesiren_slint_ui::store::ParamStore;
use slint::platform::software_renderer::RepaintBufferType;

struct Quiet;

impl HostSink for Quiet {
    fn param_changed(&self, _: ParamId, _: f32) {}
    fn gesture(&self, _: ParamId, _: bool) {}
}

const FRAMES: u32 = 120;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    /// Everything redrawn.
    Full,
    /// The Volume knob dragged in the UI: its model row and the header's MIDI-out text change.
    Knob,
    /// Only the header's text changes (a plain property, no model row).
    Text,
    /// Host automation on Volume: the store changes, the editor's poll timer refreshes the knob only.
    Host,
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn measure(scale: f32, repaint: RepaintBufferType, change: Change) -> (Duration, u32) {
    let store = Arc::new(ParamStore::default());
    let sink: Rc<dyn HostSink> = Rc::new(Quiet);
    let editor =
        EmbeddedEditor::with_repaint_buffer(&store, &sink, scale, repaint).expect("editor");
    let (w, h) = editor.size();
    let mut pixels = vec![Bgra8Premultiplied::default(); (w * h) as usize];
    editor.tick(&mut pixels, w as usize);
    let mut times = Vec::new();
    let mut area = 0;
    for i in 0..FRAMES {
        let value = (i % 128) as f32;
        match change {
            Change::Full => editor.request_full_redraw(),
            Change::Knob => editor
                .component()
                .invoke_param_changed(ParamId::Volume as i32, value),
            Change::Text => editor
                .component()
                .set_midi_out_text(format!("MIDI out: {i}").into()),
            Change::Host => {
                store.set_from_host(ParamId::Volume, value);
                std::thread::sleep(Duration::from_millis(17)); // the poll timer is due at the next tick
            }
        }
        let start = Instant::now();
        let dirty = editor.tick_region(&mut pixels, w as usize);
        times.push(start.elapsed());
        area = dirty.map_or(0, |d| d.width * d.height);
        if std::env::var_os("SHOW_DIRTY").is_some() && i == FRAMES - 1 {
            eprintln!("    last dirty rect: {dirty:?}");
        }
    }
    (median(times), area)
}

fn main() {
    println!("median of {FRAMES} ticks, software renderer into BGRA host pixels");
    let cases = [
        ("full frame", RepaintBufferType::NewBuffer, Change::Full),
        (
            "knob drag, NewBuffer",
            RepaintBufferType::NewBuffer,
            Change::Knob,
        ),
        (
            "knob drag, ReusedBuffer",
            RepaintBufferType::ReusedBuffer,
            Change::Knob,
        ),
        (
            "header text, ReusedBuffer",
            RepaintBufferType::ReusedBuffer,
            Change::Text,
        ),
        (
            "host automation, ReusedBuffer",
            RepaintBufferType::ReusedBuffer,
            Change::Host,
        ),
    ];
    for scale in [1.0, 2.0] {
        for (name, repaint, change) in cases {
            let (time, area) = measure(scale, repaint, change);
            println!("{scale}x  {name:<30} {time:>12.3?}  {area:>7} px redrawn");
        }
    }
}
