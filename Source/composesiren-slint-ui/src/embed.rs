//! The editor inside a host's own window: Slint's software renderer draws into pixels the host owns (a
//! `juce::Image` in the plugin), and the host forwards its mouse events. No second native window, no OpenGL
//! context, no event loop of Slint's own: the host's message thread drives everything through [`EmbeddedEditor::tick`].
//!
//! Partial rendering: the host keeps its pixels between ticks (a `juce::Image` does), so Slint redraws only
//! what changed (`RepaintBufferType::ReusedBuffer`) and [`EmbeddedEditor::tick_region`] says which rectangle,
//! for the host to repaint only that.
//!
//! `slint::invoke_from_event_loop` (used by Slint's live preview to reload `.slint` files, and by any other
//! thread that wants to reach the UI) queues closures that the next tick runs.
//!
//! Slint's platform is process-global and single-threaded. Every editor of the process (several plugin
//! instances in one host) is created and driven on the same thread (JUCE's message thread). Each plugin binary
//! links its own copy of Slint, so other plugins using Slint do not share this platform.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{EventLoopProxy, Platform, PointerEventButton, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, PhysicalSize};

use crate::editor::{Editor, HostSink};
use crate::store::ParamStore;

/// A pixel as `juce::Image::ARGB` stores it on little-endian machines: B, G, R, A, premultiplied.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bgra8Premultiplied {
    /// Blue.
    pub b: u8,
    /// Green.
    pub g: u8,
    /// Red.
    pub r: u8,
    /// Alpha.
    pub a: u8,
}

impl TargetPixel for Bgra8Premultiplied {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let keep = 255 - u16::from(c.alpha);
        let mix = |dst: u8, src: u8| (u16::from(dst) * keep / 255) as u8 + src;
        self.r = mix(self.r, c.red);
        self.g = mix(self.g, c.green);
        self.b = mix(self.b, c.blue);
        self.a = mix(self.a, c.alpha);
    }

    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { b, g, r, a: 255 }
    }
}

thread_local! {
    // The window the next component creation picks up (see `EmbedPlatform::create_window_adapter`).
    static NEXT_WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

type Job = Box<dyn FnOnce() + Send>;

// Closures from `slint::invoke_from_event_loop`, run by the next tick on the editor thread.
static JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());

struct JobQueue;

impl EventLoopProxy for JobQueue {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        Ok(()) // the host owns the event loop
    }

    fn invoke_from_event_loop(&self, event: Job) -> Result<(), slint::EventLoopError> {
        JOBS.lock()
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)?
            .push(event);
        Ok(())
    }
}

fn run_jobs() {
    let jobs = JOBS
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default();
    for job in jobs {
        job();
    }
}

struct EmbedPlatform {
    start: Instant,
}

impl Platform for EmbedPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        NEXT_WINDOW
            .with(|w| w.borrow_mut().take())
            .map(|w| w as Rc<dyn WindowAdapter>)
            .ok_or_else(|| slint::PlatformError::Other("no embedded window prepared".into()))
    }

    fn duration_since_start(&self) -> Duration {
        self.start.elapsed()
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(JobQueue))
    }
}

/// Install the embedding platform once per process (later calls do nothing).
pub fn install_platform() {
    let _ = slint::platform::set_platform(Box::new(EmbedPlatform {
        start: Instant::now(),
    }));
}

/// The kinds of pointer event a host forwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    /// A button went down.
    Down,
    /// A button went up.
    Up,
    /// The pointer moved (button down or not).
    Move,
    /// The pointer left the editor.
    Exit,
}

/// The editor's size in logical pixels (`OneSiren`'s `width` and `height` in `ui/onesiren.slint`, the JUCE
/// editor's 754 x 200).
pub const LOGICAL_SIZE: (f32, f32) = (754.0, 200.0);

/// Install the platform and prepare the window the next component creation picks up.
pub(crate) fn prepare_window(repaint: RepaintBufferType) -> Rc<MinimalSoftwareWindow> {
    install_platform();
    let window = MinimalSoftwareWindow::new(repaint);
    NEXT_WINDOW.with(|w| *w.borrow_mut() = Some(window.clone()));
    window
}

/// Scale and size the window once its component exists; returns the size in physical pixels.
pub(crate) fn size_window(window: &MinimalSoftwareWindow, logical: (f32, f32), scale: f32) -> PhysicalSize {
    window.dispatch_event(WindowEvent::ScaleFactorChanged {
        scale_factor: scale,
    });
    let size = PhysicalSize::new(
        (logical.0 * scale).round() as u32,
        (logical.1 * scale).round() as u32,
    );
    window.set_size(size);
    size
}

/// Run queued work, timers and animations, and redraw what changed into `pixels`.
pub(crate) fn tick_window(
    window: &MinimalSoftwareWindow,
    pixels: &mut [Bgra8Premultiplied],
    stride: usize,
) -> Option<DirtyRect> {
    run_jobs();
    slint::platform::update_timers_and_animations();
    let dirty = Cell::new(None);
    window.draw_if_needed(|renderer| {
        let region = renderer.render(pixels, stride);
        let (origin, size) = (region.bounding_box_origin(), region.bounding_box_size());
        dirty.set(Some(DirtyRect {
            x: origin.x.max(0) as u32,
            y: origin.y.max(0) as u32,
            width: size.width,
            height: size.height,
        }));
    });
    dirty.get()
}

/// Forward a pointer event at logical coordinates.
pub(crate) fn pointer_window(window: &MinimalSoftwareWindow, kind: Pointer, x: f32, y: f32) {
    let position = LogicalPosition::new(x, y);
    let button = PointerEventButton::Left;
    window.dispatch_event(match kind {
        Pointer::Down => WindowEvent::PointerPressed { position, button },
        Pointer::Up => WindowEvent::PointerReleased { position, button },
        Pointer::Move => WindowEvent::PointerMoved { position },
        Pointer::Exit => WindowEvent::PointerExited,
    });
}

/// Forward a wheel event (logical pixels).
pub(crate) fn wheel_window(window: &MinimalSoftwareWindow, x: f32, y: f32, dx: f32, dy: f32) {
    window.dispatch_event(WindowEvent::PointerScrolled {
        position: LogicalPosition::new(x, y),
        delta_x: dx,
        delta_y: dy,
    });
}

/// The part of the host's pixels a tick redrew, in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyRect {
    /// Left.
    pub x: u32,
    /// Top.
    pub y: u32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// One editor rendered into host pixels.
pub struct EmbeddedEditor {
    window: Rc<MinimalSoftwareWindow>,
    editor: Editor,
    size: PhysicalSize,
}

impl EmbeddedEditor {
    /// The editor at `scale` physical pixels per logical pixel (the host's display scale), drawing into
    /// pixels the host keeps between ticks (only what changed is redrawn).
    ///
    /// # Errors
    /// When the component cannot be created (another platform was installed first).
    pub fn new(
        store: &Arc<ParamStore>,
        sink: &Rc<dyn HostSink>,
        scale: f32,
    ) -> Result<Self, slint::PlatformError> {
        Self::with_repaint_buffer(store, sink, scale, RepaintBufferType::ReusedBuffer)
    }

    /// Like [`EmbeddedEditor::new`], choosing how Slint treats the host's pixels: `ReusedBuffer` when the host
    /// hands the same, unmodified pixels to every tick (partial redraws), `NewBuffer` to redraw everything.
    ///
    /// # Errors
    /// When the component cannot be created (another platform was installed first).
    pub fn with_repaint_buffer(
        store: &Arc<ParamStore>,
        sink: &Rc<dyn HostSink>,
        scale: f32,
        repaint: RepaintBufferType,
    ) -> Result<Self, slint::PlatformError> {
        let window = prepare_window(repaint);
        let editor = Editor::new(store, sink)?;
        let size = size_window(&window, LOGICAL_SIZE, scale);
        editor.component.show()?;
        Ok(Self {
            window,
            editor,
            size,
        })
    }

    /// The component (to call its setters directly).
    #[must_use]
    pub fn component(&self) -> &crate::OneSiren {
        &self.editor.component
    }

    /// Width and height in physical pixels: the size of the host's pixel buffer.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.size.width, self.size.height)
    }

    /// Run queued closures, timers and animations, and redraw into `pixels` (`stride` pixels per row) if
    /// anything changed. Returns true when the pixels were redrawn (the host then repaints). Call it from a
    /// host timer (60 Hz).
    pub fn tick(&self, pixels: &mut [Bgra8Premultiplied], stride: usize) -> bool {
        self.tick_region(pixels, stride).is_some()
    }

    /// Like [`EmbeddedEditor::tick`], returning the rectangle that was redrawn (the host repaints only that).
    pub fn tick_region(
        &self,
        pixels: &mut [Bgra8Premultiplied],
        stride: usize,
    ) -> Option<DirtyRect> {
        tick_window(&self.window, pixels, stride)
    }

    /// Mark the whole editor for redrawing (after the host lost its pixels, or to measure a full frame).
    pub fn request_full_redraw(&self) {
        self.window.request_redraw();
    }

    /// Forward a pointer event at logical coordinates (JUCE's component coordinates).
    pub fn pointer(&self, kind: Pointer, x: f32, y: f32) {
        pointer_window(&self.window, kind, x, y);
    }

    /// Forward a wheel event (logical pixels; positive `dy` scrolls up).
    pub fn wheel(&self, x: f32, y: f32, dx: f32, dy: f32) {
        wheel_window(&self.window, x, y, dx, dy);
    }
}
