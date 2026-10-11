//! Render both editors headless through the embedded path (the plugin's), and write PNGs:
//! `cargo run --example render_editors -- <dir> [scale]` writes onesiren-slint.png,
//! orchestra-slint-clic-on.png and orchestra-slint-clic-off.png.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use composesiren_slint_ui::editor::HostSink;
use composesiren_slint_ui::embed::{Bgra8Premultiplied, EmbeddedEditor};
use composesiren_slint_ui::orchestra::{EmbeddedOrchestra, OrchestraSink, OrchestraStore};
use composesiren_slint_ui::params::ParamId;
use composesiren_slint_ui::store::ParamStore;

struct Quiet;
impl HostSink for Quiet {
    fn param_changed(&self, _: ParamId, _: f32) {}
    fn gesture(&self, _: ParamId, _: bool) {}
}
impl OrchestraSink for Quiet {
    fn param_changed(&self, _: usize, _: f32) {}
    fn gesture(&self, _: usize, _: bool) {}
}

fn write_png(path: &Path, pixels: &[Bgra8Premultiplied], (w, h): (u32, u32)) {
    let rgba: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b, 255]).collect();
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path).expect("PNG file")), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().and_then(|mut wr| wr.write_image_data(&rgba)).expect("PNG written");
    println!("wrote {}", path.display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let scale: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    std::fs::create_dir_all(&dir).expect("output folder");

    let one = EmbeddedEditor::new(&Arc::new(ParamStore::default()), &(Rc::new(Quiet) as Rc<dyn HostSink>), scale)
        .expect("OneSiren");
    let size = one.size();
    let mut pixels = vec![Bgra8Premultiplied::default(); (size.0 * size.1) as usize];
    one.tick(&mut pixels, size.0 as usize);
    write_png(&dir.join("onesiren-slint.png"), &pixels, size);

    for clic in [true, false] {
        let store = Arc::new(OrchestraStore::new(clic));
        // a few non-default values, so the arcs show
        for (i, p) in store.params().iter().enumerate() {
            match p.meta.code_name {
                "VibratoFrequency" => store.set_from_host(i, 108.0),
                "VibratoAcceleration" => store.set_from_host(i, 97.0),
                "Volume" | "Timbre" => store.set_from_host(i, 127.0),
                "MasterVolume" => store.set_from_host(i, 0.98),
                "ReverbLowCut" => store.set_from_host(i, 20.0),
                _ => {}
            }
        }
        let sink: Rc<dyn OrchestraSink> = Rc::new(Quiet);
        let orch = EmbeddedOrchestra::new(&store, &sink, true, scale).expect("SirenOrchestra");
        let size = orch.size();
        let mut pixels = vec![Bgra8Premultiplied::default(); (size.0 * size.1) as usize];
        // two ticks: the first applies the host values (16 ms poll), the second draws them
        orch.tick_region(&mut pixels, size.0 as usize);
        std::thread::sleep(std::time::Duration::from_millis(40));
        orch.tick_region(&mut pixels, size.0 as usize);
        let name = if clic { "orchestra-slint-clic-on.png" } else { "orchestra-slint-clic-off.png" };
        write_png(&dir.join(name), &pixels, size);
    }
}
