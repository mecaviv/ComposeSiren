//! Render the embedded editor headless (the path the plugin uses), before and after a simulated drag and a
//! host automation change, and write PNGs: `cargo run --example render_png -- <dir> [scale]`.

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use composesiren_slint_ui::editor::HostSink;
use composesiren_slint_ui::embed::{Bgra8Premultiplied, EmbeddedEditor, Pointer};
use composesiren_slint_ui::params::{ParamDef, ParamId};
use composesiren_slint_ui::store::ParamStore;

#[derive(Default)]
struct Log(RefCell<Vec<String>>);

impl HostSink for Log {
    fn param_changed(&self, id: ParamId, value: f32) {
        self.0.borrow_mut().push(format!(
            "{} = {}",
            ParamDef::of(id).code_name,
            ParamDef::of(id).format(value)
        ));
    }
    fn gesture(&self, id: ParamId, begin: bool) {
        self.0.borrow_mut().push(format!(
            "{} {}",
            if begin { "begin" } else { "end" },
            ParamDef::of(id).code_name
        ));
    }
}

fn write_png(path: &Path, pixels: &[Bgra8Premultiplied], (w, h): (u32, u32)) {
    let rgba: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b, 255]).collect();
    let mut encoder =
        png::Encoder::new(BufWriter::new(File::create(path).expect("PNG file")), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut wr| wr.write_image_data(&rgba))
        .expect("PNG written");
    println!("wrote {}", path.display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let scale: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    std::fs::create_dir_all(&dir).expect("output folder");

    let store = Arc::new(ParamStore::default());
    let log = Rc::new(Log::default());
    let sink: Rc<dyn HostSink> = log.clone();
    let editor = EmbeddedEditor::new(&store, &sink, scale).expect("editor");
    let size = editor.size();
    let mut pixels = vec![Bgra8Premultiplied::default(); (size.0 * size.1) as usize];
    editor.tick(&mut pixels, size.0 as usize);
    write_png(&dir.join("onesiren-embedded.png"), &pixels, size);

    // Drag the Portamento knob up 100 px (half its range), as the host's mouse events would.
    let (x, y) = (
        std::env::var("KNOB_X")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(219.0_f32),
        108.0_f32,
    );
    editor.pointer(Pointer::Move, x, y);
    editor.pointer(Pointer::Down, x, y);
    for step in 1..=10 {
        editor.pointer(Pointer::Move, x, y - 10.0 * step as f32);
        editor.tick(&mut pixels, size.0 as usize);
    }
    editor.pointer(Pointer::Up, x, y - 100.0);
    // The host automates Volume and Timbre meanwhile.
    store.set_from_host(ParamId::Volume, 40.0);
    store.set_from_host(ParamId::Timbre, 100.0);
    std::thread::sleep(std::time::Duration::from_millis(40));
    editor.tick(&mut pixels, size.0 as usize);
    write_png(&dir.join("onesiren-embedded-after-drag.png"), &pixels, size);
    for line in log
        .0
        .borrow()
        .iter()
        .filter(|l| !l.contains(" = ") || l.starts_with("Portamento"))
        .take(3)
    {
        println!("host received: {line}");
    }
    println!(
        "Portamento = {}, Volume = {}",
        store.get(ParamId::Portamento),
        store.get(ParamId::Volume)
    );
}
