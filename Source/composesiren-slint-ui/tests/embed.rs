//! The embedded editor (the plugin path): renders into host pixels and turns host mouse events into
//! parameter changes with gestures. One test function: Slint's platform is per process and per thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use composesiren_slint_ui::editor::HostSink;
use composesiren_slint_ui::embed::{Bgra8Premultiplied, EmbeddedEditor, Pointer};
use composesiren_slint_ui::params::ParamId;
use composesiren_slint_ui::store::ParamStore;

#[derive(Default)]
struct Log(RefCell<Vec<(ParamId, Option<f32>, bool)>>);

impl HostSink for Log {
    fn param_changed(&self, id: ParamId, value: f32) {
        self.0.borrow_mut().push((id, Some(value), false));
    }
    fn gesture(&self, id: ParamId, begin: bool) {
        self.0.borrow_mut().push((id, None, begin));
    }
}

#[test]
fn renders_and_drags_a_knob() {
    let store = Arc::new(ParamStore::default());
    let log = Rc::new(Log::default());
    let sink: Rc<dyn HostSink> = log.clone();
    let editor = EmbeddedEditor::new(&store, &sink, 2.0).expect("editor");
    let (w, h) = editor.size();
    assert_eq!((w, h), (1520, 524));
    let mut pixels = vec![Bgra8Premultiplied::default(); (w * h) as usize];
    assert!(editor.tick(&mut pixels, w as usize), "first frame draws");
    assert!(
        pixels.iter().all(|p| p.a == 255),
        "opaque window background"
    );
    assert!(
        !editor.tick(&mut pixels, w as usize),
        "nothing changed, nothing redrawn"
    );

    let (x, y) = (219.0, 108.0); // the Portamento knob
    editor.pointer(Pointer::Move, x, y);
    editor.pointer(Pointer::Down, x, y);
    for step in 1..=10 {
        editor.pointer(Pointer::Move, x, y - 10.0 * step as f32);
    }
    editor.pointer(Pointer::Up, x, y - 100.0);
    let log = log.0.borrow();
    let id = log
        .first()
        .map(|e| e.0)
        .expect("the drag reached a parameter");
    assert_eq!(log.first().map(|e| e.2), Some(true), "gesture begins first");
    assert_eq!(
        log.last().map(|e| (e.0, e.2)),
        Some((id, false)),
        "gesture ends last"
    );
    assert!(
        store.get(id) > 50.0,
        "{id:?} moved by half its range: {}",
        store.get(id)
    );

    store.set_from_host(ParamId::Volume, 10.0);
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert!(editor.tick(&mut pixels, w as usize), "host change redraws");
}
