//! The editor inside a host's own window: Slint's software renderer draws into pixels the host owns (a
//! `juce::Image` in the plugin), and the host forwards its mouse events. No second native window, no OpenGL
//! context, no event loop of Slint's own: the host's message thread drives everything through [`EmbeddedEditor::tick`].
//!
//! Slint's platform is process-global and single-threaded. Every editor of the process (several plugin
//! instances in one host) is created and driven on the same thread (JUCE's message thread). Each plugin binary
//! links its own copy of Slint, so other plugins using Slint do not share this platform.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{Platform, PointerEventButton, WindowAdapter, WindowEvent};
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

/// The editor's size in logical pixels (`OneSiren`'s `width` and `height` in `ui/onesiren.slint`).
pub const LOGICAL_SIZE: (f32, f32) = (760.0, 262.0);

/// One editor rendered into host pixels.
pub struct EmbeddedEditor {
    window: Rc<MinimalSoftwareWindow>,
    editor: Editor,
    size: PhysicalSize,
}

impl EmbeddedEditor {
    /// The editor at `scale` physical pixels per logical pixel (the host's display scale).
    ///
    /// # Errors
    /// When the component cannot be created (another platform was installed first).
    pub fn new(
        store: &Arc<ParamStore>,
        sink: &Rc<dyn HostSink>,
        scale: f32,
    ) -> Result<Self, slint::PlatformError> {
        install_platform();
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        NEXT_WINDOW.with(|w| *w.borrow_mut() = Some(window.clone()));
        let editor = Editor::new(store, sink)?;
        window.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
        let size = PhysicalSize::new(
            (LOGICAL_SIZE.0 * scale).round() as u32,
            (LOGICAL_SIZE.1 * scale).round() as u32,
        );
        window.set_size(size);
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

    /// Run timers and animations, and redraw into `pixels` (`stride` pixels per row) if anything changed.
    /// Returns true when the pixels were redrawn (the host then repaints). Call it from a host timer (60 Hz).
    pub fn tick(&self, pixels: &mut [Bgra8Premultiplied], stride: usize) -> bool {
        slint::platform::update_timers_and_animations();
        self.window.draw_if_needed(|renderer| {
            renderer.render(pixels, stride);
        })
    }

    /// Forward a pointer event at logical coordinates (JUCE's component coordinates).
    pub fn pointer(&self, kind: Pointer, x: f32, y: f32) {
        let position = LogicalPosition::new(x, y);
        let button = PointerEventButton::Left;
        self.window.dispatch_event(match kind {
            Pointer::Down => WindowEvent::PointerPressed { position, button },
            Pointer::Up => WindowEvent::PointerReleased { position, button },
            Pointer::Move => WindowEvent::PointerMoved { position },
            Pointer::Exit => WindowEvent::PointerExited,
        });
    }

    /// Forward a wheel event (logical pixels; positive `dy` scrolls up).
    pub fn wheel(&self, x: f32, y: f32, dx: f32, dy: f32) {
        self.window.dispatch_event(WindowEvent::PointerScrolled {
            position: LogicalPosition::new(x, y),
            delta_x: dx,
            delta_y: dy,
        });
    }
}
