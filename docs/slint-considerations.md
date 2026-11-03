# Slint UI: iteration speed and rendering performance

Companion to [slint-port-investigation.md](slint-port-investigation.md). JUCE keeps the audio, the parameters
and the plugin formats; the questions here are **how fast the UI can be iterated on** and **how fast it
draws**, and what to change if the current way of drawing (Slint's software renderer into the JUCE editor's
image) is not good enough.

All measurements are from the build box (Linux x86_64, 8 cores), Release builds unless noted.
Reproduce the render numbers with `cargo run --release --example frame_time` in
`Source/composesiren-slint-ui`.

## 1. Fast rebuilds and hot reload

### What Slint offers

| Tool | What it does | Use here |
|---|---|---|
| **VS Code extension / Slint LSP live preview** | Renders the `.slint` file being edited, updates on every keystroke, property editor, outline. | Layout and look work on `ui/*.slint` without building anything. The strip comes from a Rust model, so the preview shows an empty strip unless the component gets preview data (see below). |
| **`slint-viewer`** (`cargo install slint-viewer`) | Same renderer as a standalone window; `--auto-reload`, `--load-data file.json` to fill properties and models. | A `preview.json` with the 14 parameter rows would make the strip visible in the viewer and the LSP preview. A small next step. |
| **Live preview in the application** (Slint ≥ 1.13): build with `SLINT_LIVE_PREVIEW=1 cargo build --features slint/live-preview` | The `slint!`/`slint-build` code generation is replaced by stubs with the same Rust API that load the `.slint` files at run time with the interpreter, watch them, and reload them. Properties, callbacks and models set from Rust survive the reload. Renaming a property or callback used from Rust needs a rebuild. | **Works inside the plugin path.** Verified on the box: with the embedded editor ticking (the JUCE path, no winit), editing the strip background in `ui/onesiren.slint` redrew the running editor in red within a tick, and back again, without restarting. This needed one change, now in the PoC: the embedding platform provides an event-loop proxy (`slint::invoke_from_event_loop` closures run at the next JUCE tick), which is how the live preview hands reloads to the UI thread. |
| **`slint-interpreter` directly** | Load any `.slint` at run time, set/get properties and callbacks by name (`ComponentDefinition`, `ComponentInstance`). | What the live preview uses under the hood. Only worth using directly to load user-provided UIs (skins), not for development. |

Recommended setup:
- **Development builds:** a CMake cache option, for example `COMPOSESIREN_SLINT_LIVE_PREVIEW` (default OFF,
  never in release packages). It sets `SLINT_LIVE_PREVIEW=1` and `--features slint/live-preview` for the
  crate's cargo invocation (the Rust staticlib helper already passes FEATURES and environment variables).
  Not added in this PoC: it touches the shared CMake helper. Today it works with cargo directly.
- **Release builds:** keep the compiled `slint-build` output, with no interpreter in the binary. The
  live-preview build is bigger and slower, and it reads `.slint` files from the source tree.
- Notes:
  - The first live-preview build compiled the interpreter in 47 s (Release, 8 cores); after that it is
    incremental.
  - Editors that save by renaming a new file over the old one may not be picked up. The watcher saw
    in-place writes reliably; `sed -i`, which renames, was missed once.

### Rebuild cycle times measured here

| Change | Command | Time |
|---|---|---|
| Edit `ui/onesiren.slint` | `cargo build` (debug, crate) | 1.5 s |
| Edit `ui/onesiren.slint` | `cargo build --release --lib` (the staticlib the plugin links) | 4.3 s |
| Edit a Rust file of the crate | `cargo build` (debug) | 1.5 s |
| Edit `ui/onesiren.slint` with live preview running | none: reload | under a second, no restart |
| Edit `SirenStripComponent.cpp` (JUCE UI) | `ninja OneSiren_Standalone` (Release + JUCE's recommended LTO flags) | 111 s |

Both plugin paths pay the same relink (LTO dominates the JUCE number; a Debug configuration without LTO
links much faster). The difference is that Slint can skip the rebuild entirely with live preview.

### JUCE, for comparison

- **No declarative UI and no hot reload of components.** Layout and look are C++ (`resized()`, `paint()`,
  `LookAndFeel` classes); every change is recompile, relink, restart.
- `juce_gui_extra`'s **live constant editor** (`JUCE_LIVE_CONSTANT(x)`) lets you tweak numeric and colour
  literals at run time in Debug builds. It is useful for tuning a colour or an offset, but it can't change
  structure, and the values must be copied back into the code by hand.
- The **Projucer live build** (the C++ JIT "live coding" engine) was removed years ago, so it is not an option.
- **JIVE** (a third-party JUCE module) adds declarative XML/`ValueTree` UIs and style sheets with some
  run-time reload. That is an extra dependency, and the repo's build doesn't use it.
- **Hot-reloading a C++ editor DLL** is possible in principle, but nobody does it for JUCE plugins in
  practice.

Iteration speed is one of Slint's clearest wins over the JUCE components: seconds without the plugin build,
and no rebuild with live preview.

## 2. Redrawing and performance

### How the PoC draws

1. A JUCE `Timer` at 60 Hz calls `cs_slint_ui_tick_region`.
2. Slint runs timers and animations. If something changed, its **software renderer rasterises on the CPU**
   straight into the editor's `juce::Image`: a software ARGB image whose memory layout (BGRA,
   premultiplied) is the pixel type the renderer writes, so there is no conversion and no copy on our side.
3. The editor calls `repaint()` on the redrawn rectangle. `paint()` draws the image. At 2x the image is
   already at physical resolution, so JUCE blits it 1:1 into the 2x backing store.

### Why it can be slower than the JUCE components

- **CPU rasterisation of everything Slint draws.** JUCE also renders components on the CPU on macOS
  (CoreGraphics) and Linux. On Windows, JUCE 8 uses Direct2D (GPU). So the difference from JUCE is smaller
  than "CPU vs GPU" suggests, except on Windows.
- **Area grows with the square of the scale.** A full frame of OneSiren is 199k pixels at 1x and 796k at
  2x, and the time roughly doubles (below).
- **The upload to the OS on every repaint.** JUCE turns the software image into a native image when it
  paints (`CGImage` on macOS, a D2D bitmap on Windows), so every repaint moves the repainted part of the
  image again. Repainting only the dirty rectangle bounds that.
- **Ticking at 60 Hz.** A tick with nothing to do costs only the timer callback: Slint draws only when a
  property it depends on changed, or an animation runs.
- **Text shaping** is done by Slint's own text stack: fonts are discovered (CoreText/fontconfig) and shaped
  in software. It is cached after the first frame.

### Measurements (median of 120 ticks, embedded path, Release)

| Case | 1x | pixels redrawn | 2x | pixels redrawn |
|---|---|---|---|---|
| Full frame (`NewBuffer`, or a forced full redraw) | 0.79 ms | 199,120 | 1.60 ms | 796,480 |
| Knob dragged in the UI, full redraws (`NewBuffer`) | 1.49 ms | 199,120 | 2.29 ms | 796,480 |
| Knob dragged in the UI, partial redraws (`ReusedBuffer`) | 1.17 ms | 126,198 | 1.58 ms | 504,792 |
| Header text only, partial | 0.22 ms | 10,088 | 0.23 ms | 40,352 |
| Host automation on one knob, partial (store, then poll, then row) | 1.47 ms | 87,084 | 1.85 ms | 348,336 |

These are per changed frame. At 60 Hz that is at most about 2 ms of the message thread's 16.7 ms, and only
while something moves. It is fine for OneSiren. SirenOrchestra (7 strips, about 4.5 times the area) should
be measured before committing to this path at 2x.

What the numbers taught, and what is now in the PoC:
- **Model granularity matters more than pixels.** With one flat parameter model indexed by position
  (`root.params[i]`), any row change made every cell re-evaluate: 2.1 ms for a knob change. One model per
  group of the strip, with a repeater over each, brought it to 1.2 ms at 1x. That is about as long as a full
  frame: property and layout evaluation costs as much as rasterising.
- **Partial redraws (`RepaintBufferType::ReusedBuffer`)** are now the default: the JUCE image keeps its
  pixels between ticks, so Slint redraws only the dirty region. `tick_region` reports the rectangle, and
  the editor repaints only that, which also bounds the upload to the OS.
- **Still open:** a single knob change still dirties the whole row of strip cells (x 11–749, y 57–175 at
  1x), not just that knob. Drop shadows and the value text's width were ruled out. The remaining suspect is
  the strip's nested layouts: any change in a repeated cell re-runs the group and strip layout, and the
  partial renderer counts re-laid-out items as dirty. Next experiments:
  - fixed-geometry cells (explicit `x`/`width` instead of `HorizontalLayout`/`VerticalLayout` for the
    cells);
  - `PhysicalRegion::iter()` instead of the bounding box, to see the individual rectangles;
  - asking upstream.
  The interaction of UI drags with the header's MIDI-out text also widens the box. Showing that text in
  the strip cell, or reporting several rectangles, avoids it.

### Further mitigations within the software path

- **Repaint several rectangles:** `renderer.render` returns a `PhysicalRegion` with up to a few
  rectangles. Hand them all to JUCE (`repaint` per rectangle) instead of their bounding box.
- **Cadence:**
  - Tick at the display rate (JUCE `VBlankAttachment`, JUCE 7+) instead of a fixed 60 Hz `Timer`.
  - Stop ticking when the editor is hidden or minimised.
  - Wake the editor for host changes with `invoke_from_event_loop` (now supported) instead of polling.
- **Pixel format:** already native (`juce::SoftwareImageType`, BGRA premultiplied). Do not use the default
  native image type: on Windows it is a Direct2D image, and `BitmapData` would round-trip it through the
  GPU on every tick.
- **Scale:** render at the display's scale (done) and re-create the image when the scale changes (to do;
  `ComponentPeer`/`Desktop` scale listeners).
- **Line-by-line rendering** (`render_by_line`) only helps memory-constrained targets. Not useful here.

### What the Slint showcase audio plugins do

A survey of the Slint showcase ([slint-showcase-rendering.md](slint-showcase-rendering.md), 2026-10-05)
found no open plugin code that uses the software renderer in a host window:

- The two showcase DAW plugins, [WesAudio](https://slint.dev/success/wesaudio-daw.html) and
  [Viiri Audio Aava](https://slint.dev/success/viiri-audio.html), are closed source.
- Aava's author publishes [plugin-things](https://github.com/ilmai/plugin-things) (MIT):
  `plugin-canvas` and `plugin-canvas-slint`. That Aava uses it is an inference from shared authorship.
  - It creates a native child window (`WS_CHILD` HWND, NSView, X11).
  - A custom Slint `Platform` and `WindowAdapter` drive the **Skia GPU renderer**: Direct3D on Windows,
    Metal on macOS 13+ (OpenGL before), OpenGL on Linux.
  - `request_redraw()` only sets a flag; `render()` runs on the next frame tick only when it is set.
  - Ticks are vsync-paced: DXGI `WaitForVBlank` on Windows, `CADisplayLink` on macOS 14+, a 16 ms host
    timer on Linux.
- [slint-baseview](https://codeberg.org/RustAudio/slint-baseview) and
  [nice-plug-slint](https://github.com/aidan729/nice-plug-slint) use FemtoVG on OpenGL and render
  **every** 15 ms tick unconditionally, with vsync off. Not a performance model.
- Slint's C++ [`platform_native`](https://github.com/slint-ui/slint/blob/master/examples/cpp/platform_native/windowadapter_win.h)
  example does the same with `slint::platform::SkiaRenderer` in a `WS_CHILDWINDOW`: `request_redraw()` is
  `InvalidateRect`, and `WM_PAINT` renders.
- Slint discussions: [#4691](https://github.com/slint-ui/slint/discussions/4691) (GPU backends repaint the
  full window) and [#5677](https://github.com/slint-ui/slint/discussions/5677) (FemtoVG to Skia cut CPU
  from about 30 % to 2 %, for a binary of about 20 MB instead of 3 MB).

The survey's software-renderer checklist, against the PoC:

| Recommendation | PoC |
|---|---|
| `RepaintBufferType::ReusedBuffer` and one persistent buffer | done (`embed.rs`; one `juce::Image` per editor) |
| Repaint only the region `render()` returns, in logical coordinates | done (bounding box; several rectangles to do, above) |
| Render straight into the JUCE image memory through `TargetPixel` | done (`Bgra8Premultiplied` written through `Image::BitmapData`, no conversion pass) |
| `juce::SoftwareImageType`, not the native (Direct2D) image | done |
| Buffer in physical pixels, scale passed to Slint (`ScaleFactorChanged`), so JUCE never resamples | done at creation; re-creating on scale change is to do |
| Tick from `juce::VBlankAttachment`, render only when Slint asked | render-on-request done (`draw_if_needed`); `VBlankAttachment` to do (60 Hz `Timer` now) |
| Apply host values once per tick, not per audio callback | done (lock-free store read at tick) |

What stays expensive on this path is continuous animation over large areas (meters, the SirenWaves
cells, spectra) and big editors at 2x: that is when to move to a GPU renderer.

### Alternatives that keep JUCE

| Option | How | Gains | Costs and risks |
|---|---|---|---|
| **A. Software renderer into `juce::Image`** (now) | As above | Works in every host and format; no native child window; no GPU context; headless tests | CPU per changed pixel; the upload to the OS on repaint |
| **B. Slint FemtoVG (OpenGL) on a JUCE `OpenGLContext`** | Attach a `juce::OpenGLContext` to the editor; a custom Slint `WindowAdapter` whose renderer is `slint::platform::femtovg_renderer::FemtoVGRenderer::new(impl OpenGLInterface)`, where `ensure_current`, `swap_buffers` and `get_proc_address` forward to JUCE's context (`juce::OpenGLHelpers::getExtensionFunction`); call `renderer.render()` from `renderOpenGL()` | GPU raster; no CPU-to-OS upload; scales well at 2x and for SirenOrchestra | JUCE renders GL on **its own thread**, but Slint is single-threaded: the whole Slint UI must then live on the GL thread (the lock-free Rust store makes that workable, and input events must be queued to it). OpenGL is deprecated on macOS. Contexts per editor instance, and GL state shared with JUCE's own GL drawing. Needs the `renderer-femtovg` feature. |
| **C. Skia renderer** (`renderer-skia`: Metal on macOS, D3D on Windows, Vulkan/GL on Linux) | Same custom-adapter pattern; Skia's Metal/D3D surfaces need the native view or layer | Best quality and speed; Metal on macOS (no deprecated GL) | Large dependency (Skia binaries, build time); surfaces bound to a native view, so effectively option D on macOS and Windows |
| **D. Native child window parented under JUCE's peer, Skia GPU** (the pattern of the showcase plugin code) | **D1, fastest to try:** `plugin-canvas` + `plugin-canvas-slint`, with `getPeer()->getNativeHandle()` (or a `juce::HWNDComponent` / `NSViewComponent` / `XEmbedComponent`) as the parent window handle; on Linux call its `on_frame()` from a 16 ms `juce::Timer`; forward JUCE scale changes to `set_scale()`. **D2, own the adapter:** a `WindowAdapter` of about 300 lines modelled on `plugin-canvas-slint/src/window_adapter.rs` or Slint's C++ `platform_native` example: child view, `SkiaRenderer` (`default_direct3d` / `default_metal`), JUCE mouse and key events translated to `WindowEvent`s, driven by `juce::VBlankAttachment` (queued closures, `update_timers_and_animations()`, then `render()` only when `request_redraw` set the flag). Slint's C++ `slint::platform::SkiaRenderer` lets D2 live in C++. | GPU raster straight into a native surface: no CPU raster, no copy, no second blit by JUCE; vsync-paced, redraw on request; Metal on macOS. Shipping evidence: Aava (inferred). | Child windows inside host windows are where plugin UIs break: focus and keyboard routing, resize, DPI changes, z-order, per host and per OS. D1 depends on Slint's internal `i-slint-*` crates pinned to `~1.17.1` (breaks on Slint upgrades) and uses one global Slint platform per process. Skia makes the binary much bigger. Avoid `slint-baseview` as a model (renders every 15 ms, GL, vsync off). |
| **E. Event loop integration (applies to every option)** | A custom Slint `Platform` driven by the JUCE message thread: `update_timers_and_animations()` from a JUCE timer, `duration_until_next_timer_update()` to schedule the next tick, and an `EventLoopProxy` for `invoke_from_event_loop` | No second event loop; Slint timers and animations run on JUCE's thread | Already done for A in the PoC; B needs the same on the GL thread |

Recommendation:
- Stay on **A** for OneSiren: with partial redraws it is 0.2 to 1.6 ms per change (measured above). Finish
  its checklist items: `VBlankAttachment`, several dirty rectangles, image re-creation on scale changes.
- Prototype SirenOrchestra's 7 strips at 2x on A (the `frame_time` example extended to that layout), with
  and without a continuously animating cell, before porting it.
- If that is too slow, move to a GPU renderer:
  - **D** (Skia in a child window) is the route the showcase plugin code proves. Benchmark **D1**
    (`plugin-canvas-slint`) against A on the same UI at 1x and 2x, then write **D2** (own adapter,
    `VBlankAttachment`) for production, to avoid the pinned internal crates.
  - **B** stays the option without child windows, if host child-window problems show up. Mind its GL
    thread and macOS's deprecated OpenGL.

## 3. Shared interface metadata

Both editors need the same facts: parameter code names, labels, strip captions, ranges, steps,
defaults, CC numbers, widgets, sections, strip colours, the palette and the strip layout. They are
now generated, not copied by hand:

- `Source/ComposeSirenCore/lib/definitions/generated/UiMetadata.h` (namespace
  `mecaviv::metadata::ui`) for the JUCE editors;
- `Source/composesiren-slint-ui/ui/generated/metadata.slint` (the `UiMetadata` and `Theme` globals)
  and `src/generated/metadata.rs` (the `metadata` module) for the Slint editor.

The three files are committed, carry a "GENERATED, do not edit" notice and are produced from
metadata tables maintained outside this repository; the build doesn't need anything else. The
Slint editor already takes its strip grey and orange from `Theme`, and tests in `src/params.rs`
check its hand-written parameter table, groups and colour ramp against `metadata.rs`. Next steps:
- make `PARAMS` and `GROUPS` views over `metadata::PARAMETERS`;
- on the JUCE side, build `parameterDefinitions`, the strip captions, `palette.h` and
  `controlStripLayout` from `UiMetadata.h`.

## 4. The SirenWaves shader

How the SirenWaves shader of PR 32 (`Assets/shaders/SirenWave.frag`, `juce::OpenGLGraphicsContextCustomShader`)
could serve the JUCE build and a Slint editor from one source is assessed separately, outside this
repository. In short:
- Slint has no custom shaders yet. While JUCE hosts the editor, the wave cells can stay JUCE components
  over transparent Slint cells.
- The shader's CPU twin (`SirenWaveMath.h`, `SirenWaveRaster.h`) can render the cells into a Slint
  `Image` when there is no OpenGL context.
- A FemtoVG-rendered Slint could draw the same GLSL in an OpenGL underlay.
