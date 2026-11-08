//! GPU rasterization while preserving the host's pixel-buffer contract.
//! A persistent BGRA texture and staging buffer keep the image transfer allocation-free between resizes.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use slint::platform::femtovg_renderer::FemtoVGWGPURenderer;
use slint::platform::{Renderer, WindowAdapter, WindowEvent};
use slint::wgpu_30::wgpu;
use slint::{PhysicalSize, Window, WindowSize};

use super::{Bgra8Premultiplied, DirtyRect};

fn wait<F: Future>(future: F) -> F::Output {
    struct Notify(std::thread::Thread);
    impl Wake for Notify {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Notify(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// Host-driven GPU window. The adapter and its component stay on the host's GUI thread.
pub struct GpuWindow {
    window: Window,
    renderer: FemtoVGWGPURenderer,
    device: wgpu::Device,
    queue: wgpu::Queue,
    frame: RefCell<Option<Frame>>,
    size: Cell<PhysicalSize>,
    dirty: Cell<bool>,
    failed: Cell<bool>,
}

struct Frame {
    size: PhysicalSize,
    texture: wgpu::Texture,
    readback: wgpu::Buffer,
    row_bytes: u32,
}

impl Frame {
    fn new(device: &wgpu::Device, size: PhysicalSize) -> Self {
        let row_bytes = (size.width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        Self {
            size,
            row_bytes,
            texture: device.create_texture(&wgpu::TextureDescriptor {
                label: Some("mecaviv Slint panel"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mecaviv Slint host image transfer"),
                size: u64::from(row_bytes) * u64::from(size.height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }
}

impl GpuWindow {
    pub fn new() -> Result<Rc<Self>, slint::PlatformError> {
        let instance = wgpu::Instance::default();
        let adapter = wait(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        }))
        .map_err(|error| slint::PlatformError::Other(error.to_string()))?;
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            return Err(slint::PlatformError::Other(
                "Only a CPU WGPU adapter is available".into(),
            ));
        }
        let (device, queue) = wait(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|error| slint::PlatformError::Other(error.to_string()))?;
        let renderer = FemtoVGWGPURenderer::new(instance, device.clone(), queue.clone())?;
        eprintln!(
            "Slint GPU rasterizer: {} ({:?}); host image readback",
            info.name, info.backend
        );
        Ok(Rc::new_cyclic(|weak: &std::rc::Weak<Self>| Self {
            window: Window::new(weak.clone()),
            renderer,
            device,
            queue,
            frame: RefCell::new(None),
            size: Cell::new(PhysicalSize::new(0, 0)),
            dirty: Cell::new(true),
            failed: Cell::new(false),
        }))
    }

    pub fn draw(&self, pixels: &mut [Bgra8Premultiplied], stride: usize) -> Option<DirtyRect> {
        if self.failed.get() || !self.dirty.replace(false) {
            return None;
        }
        let size = self.size.get();
        let width = size.width as usize;
        let height = size.height as usize;
        if width == 0
            || height == 0
            || stride < width
            || height
                .checked_mul(stride)
                .is_none_or(|length| pixels.len() < length)
        {
            self.dirty.set(true);
            return None;
        }
        match self.render_frame(size, pixels, stride) {
            Ok(()) => Some(DirtyRect {
                x: 0,
                y: 0,
                width: size.width,
                height: size.height,
            }),
            Err(error) => {
                self.failed.set(true);
                eprintln!("Slint GPU frame failed: {error}");
                None
            }
        }
    }

    fn render_frame(
        &self,
        size: PhysicalSize,
        pixels: &mut [Bgra8Premultiplied],
        stride: usize,
    ) -> Result<(), slint::PlatformError> {
        let error = |error: String| slint::PlatformError::Other(error);
        let limit = self.device.limits().max_texture_dimension_2d;
        if size.width > limit || size.height > limit {
            return Err(error(format!(
                "Panel size exceeds GPU texture limit {limit}"
            )));
        }
        let mut frame = self.frame.borrow_mut();
        if frame.as_ref().is_none_or(|frame| frame.size != size) {
            *frame = Some(Frame::new(&self.device, size));
        }
        let frame = frame.as_ref().expect("frame allocated above");
        self.renderer.render_to_texture(&frame.texture)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            frame.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &frame.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(frame.row_bytes),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = frame.readback.slice(..);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| error(e.to_string()))?;
        receive
            .recv()
            .map_err(|e| error(e.to_string()))?
            .map_err(|e| error(e.to_string()))?;
        let mapped = slice.get_mapped_range().map_err(|e| error(e.to_string()))?;
        let width_bytes = size.width as usize * 4;
        for (source, target) in mapped
            .chunks_exact(frame.row_bytes as usize)
            .zip(pixels.chunks_exact_mut(stride))
        {
            // repr(C), four u8 fields; length and stride were validated by draw().
            // GPU output and host image are both premultiplied BGRA, so no conversion is needed.
            let bytes = unsafe {
                std::slice::from_raw_parts_mut(target.as_mut_ptr().cast::<u8>(), width_bytes)
            };
            bytes.copy_from_slice(&source[..width_bytes]);
        }
        drop(mapped);
        frame.readback.unmap();
        Ok(())
    }
}

impl WindowAdapter for GpuWindow {
    fn window(&self) -> &Window {
        &self.window
    }
    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
    fn size(&self) -> PhysicalSize {
        self.size.get()
    }
    fn set_size(&self, size: WindowSize) {
        self.size.set(size.to_physical(self.window.scale_factor()));
        self.window.dispatch_event(WindowEvent::Resized {
            size: size.to_logical(self.window.scale_factor()),
        });
        self.dirty.set(true);
    }
    fn request_redraw(&self) {
        self.dirty.set(true);
    }
}
