//! The `OneSiren` editor in its own window, with a simulated host that prints what the plugin would receive.
//!
//! `cargo run` shows the editor; `cargo run -- --automate` also moves Vibrato Depth from another thread, the
//! way host automation does, to show host -> editor updates through the lock-free store.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use composesiren_slint_ui::editor::{Editor, HostSink};
use composesiren_slint_ui::midi;
use composesiren_slint_ui::params::{CATEGORIES, ParamDef, ParamId};
use composesiren_slint_ui::store::ParamStore;
use slint::ComponentHandle;

struct PrintingHost;

impl HostSink for PrintingHost {
    fn param_changed(&self, id: ParamId, value: f32) {
        let def = ParamDef::of(id);
        let midi = midi::message(def, value, 1)
            .map(midi::describe)
            .unwrap_or_default();
        println!(
            "S5 | {:<20} = {:>8}   {midi}",
            def.code_name,
            def.format(value)
        );
    }
    fn gesture(&self, id: ParamId, begin: bool) {
        println!(
            "{} gesture on S5 | {}",
            if begin { "begin" } else { "end  " },
            ParamDef::of(id).code_name
        );
    }
    fn category_changed(&self, category: usize) {
        println!("siren type: {}", CATEGORIES[category].0);
    }
    fn note(&self, note: u8, on: bool) {
        println!("note {note} {}", if on { "on" } else { "off" });
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let store = Arc::new(ParamStore::default());
    let sink: Rc<dyn HostSink> = Rc::new(PrintingHost);
    let editor = Editor::new(&store, &sink)?;
    if std::env::args().any(|a| a == "--automate") {
        std::thread::spawn(move || {
            let start = Instant::now();
            loop {
                let t = start.elapsed().as_secs_f32();
                store.set_from_host(ParamId::VibratoAmplitude, 63.5 + 63.5 * (t * 1.5).sin());
                std::thread::sleep(Duration::from_millis(20));
            }
        });
    }
    editor.component.run()
}
