#![cfg(feature = "gpu")]
//! Explicit hardware test: `cargo test --features gpu --test gpu -- --ignored --nocapture`.

use composesiren_slint_ui::{
    editor::HostSink,
    embed::{Bgra8Premultiplied, EmbeddedEditor, Pointer},
    orchestra::{EmbeddedOrchestra, OrchestraSink, OrchestraStore},
    params::ParamId,
    store::ParamStore,
};
use slint::ComponentHandle;
use slint::platform::software_renderer::RepaintBufferType;

slint::slint! {
    export component TransparentProbe inherits Window {
        width: 20px; height: 20px; background: transparent;
        Rectangle { background: #ff000080; }
    }
}
use std::{rc::Rc, sync::Arc};

struct Quiet;
impl HostSink for Quiet {
    fn param_changed(&self, _: ParamId, _: f32) {}
    fn gesture(&self, _: ParamId, _: bool) {}
}
impl OrchestraSink for Quiet {
    fn param_changed(&self, _: usize, _: f32) {}
    fn gesture(&self, _: usize, _: bool) {}
}

#[test]
#[ignore = "requires a hardware GPU"]
fn standalone_gpu_renders_and_accepts_input_while_plugins_remain_software() {
    let store = Arc::new(ParamStore::default());
    let sink: Rc<dyn HostSink> = Rc::new(Quiet);
    let gpu =
        EmbeddedEditor::with_renderer(&store, &sink, 2.0, RepaintBufferType::ReusedBuffer, true)
            .unwrap();
    let expected_gpu = std::env::var("MECAVIV_SLINT_RENDERER").as_deref() != Ok("software");
    assert_eq!(gpu.is_gpu(), expected_gpu);
    let (w, h) = gpu.size();
    let mut pixels = vec![Bgra8Premultiplied::default(); (w * h) as usize];
    assert!(gpu.tick(&mut pixels, w as usize));
    assert!(pixels.iter().all(|p| p.a == 255));
    assert!(pixels.windows(2).any(|p| p[0].r != p[1].r));
    gpu.pointer(Pointer::Down, 219.0, 94.0);
    gpu.pointer(Pointer::Move, 219.0, -6.0);
    gpu.pointer(Pointer::Up, 219.0, -6.0);
    assert!(store.get(ParamId::Portamento) > 50.0);
    assert!(gpu.tick(&mut pixels, w as usize));

    let plugin = EmbeddedEditor::new(&store, &sink, 1.0).unwrap();
    assert!(
        !plugin.is_gpu(),
        "even with GPU code linked, a plugin uses software"
    );

    let orch_store = Arc::new(OrchestraStore::new(true));
    let orch_sink: Rc<dyn OrchestraSink> = Rc::new(Quiet);
    let orchestra =
        EmbeddedOrchestra::with_renderer(&orch_store, &orch_sink, true, 1.0, true).unwrap();
    assert_eq!(orchestra.is_gpu(), expected_gpu);
    let (w, h) = orchestra.size();
    let mut pixels = vec![Bgra8Premultiplied::default(); (w * h) as usize];
    assert!(orchestra.tick_region(&mut pixels, w as usize).is_some());
    assert!(pixels.iter().all(|p| p.a == 255));
    if expected_gpu {
        // Host images use premultiplied alpha; catch an accidental second multiplication on readback.
        let window = composesiren_slint_ui::embed::prepare_window_with_gpu(
            RepaintBufferType::ReusedBuffer,
            true,
        );
        let probe = TransparentProbe::new().unwrap();
        let _ = composesiren_slint_ui::embed::size_window(&window, (20.0, 20.0), 1.0);
        probe.show().unwrap();
        let mut alpha_pixels = vec![Bgra8Premultiplied::default(); 400];
        composesiren_slint_ui::embed::tick_window(&window, &mut alpha_pixels, 20).unwrap();
        let middle = alpha_pixels[210];
        assert!((i16::from(middle.a) - 128).abs() <= 1, "{middle:?}");
        assert!((i16::from(middle.r) - 128).abs() <= 1, "{middle:?}");
    }
}
