# How the Slint showcase projects render (with a focus on audio plugins)

Researched on 2026-10-05. Sources: https://slint.dev/showcase and the linked repos, cloned locally. File:line links point at the commits listed at the end.

## TL;DR

- **The showcase has two real DAW plugins: WesAudio (_HYPERION/_PROMETHEUS) and Viiri Audio "Aava". Both are closed source.** The customer stories don't name a renderer or embedding method. WesAudio says it left VSTGUI because it wanted "a responsive GUI framework that could utilise the GPU". Nothing public says either one uses Slint's software renderer.
- **Aava's author wrote the most useful open code.** Jussi Viiri (Viiri Audio) publishes `plugin-things` (MIT): `plinth-plugin` (CLAP/VST3/AUv3), `plugin-canvas` (windowing) and `plugin-canvas-slint`. It's the closest public evidence of how Aava probably renders. That's an inference: Aava itself is closed, but the crates carry `jussi@viiri-audio.com`, and a Slint maintainer pointed plugin developers to this repo ([discussion #6585](https://github.com/slint-ui/slint/discussions/6585)). Here is how it works:
  - It creates a **native child window** (Win32 `WS_CHILD` HWND / NSView / X11) inside the host's parent window.
  - It implements a **custom Slint `Platform` + `WindowAdapter`**.
  - It renders with the **Skia GPU renderer**: Direct3D on Windows, Metal on macOS 13+ (OpenGL on older macOS), and Skia's default (OpenGL) on Linux.
  - It only redraws when Slint asked for a redraw.
  - Frames are **vsync-paced** on Windows (DXGI `WaitForVBlank`) and come from `CADisplayLink` on macOS 14+.
- **The other open audio entries are standalone apps on Slint's stock winit backend**, so they don't show anything about embedding:
  - AccSyn: FemtoVG/OpenGL by default.
  - Zeedle: Skia.
  - Chiptrack: FemtoVG; the GBA build uses the software renderer.
- **Helper crates `slint-baseview` / `nice-plug-slint` use a different approach:** baseview + FemtoVG over OpenGL, rendering **every** 15 ms tick unconditionally, with vsync off.
- **None of the open-source plugin code here uses the software renderer in a host window** (for WesAudio we can't tell). For your JUCE PoC, the GPU child-window approach is the proven one. If you keep the software renderer, it has to do partial rendering into a reused buffer (details at the end).

## Audio projects and plugin helper crates

| Project | Type | Plugin framework | Embedding | Slint renderer | Redraw strategy | Source |
|---|---|---|---|---|---|---|
| **WesAudio _HYPERION / _PROMETHEUS** | DAW plugin (controls hardware), proprietary | Not stated. They left VSTGUI and say JUCE/iPlug2 were "out of scope" | Not disclosed | Not disclosed. They say they wanted GPU use | Not disclosed | [customer story](https://slint.dev/success/wesaudio-daw.html) |
| **Viiri Audio "Aava"** | Convolution plugin, proprietary | Own Rust stack. The author's public `plinth-plugin` covers CLAP/VST3/AUv3 (inference) | Probably `plugin-canvas` native child window (inference) | Probably Skia GPU via `plugin-canvas-slint` (inference) | See plugin-canvas-slint below | [customer story](https://slint.dev/success/viiri-audio.html) |
| **plugin-things → plugin-canvas-slint** (Jussi Viiri, MIT) | Plugin framework + Slint bridge | `plinth-plugin` (CLAP, VST3, AUv3) | Custom `Platform`/`WindowAdapter` on a `plugin-canvas` child window: Win32 `WS_CHILD\|WS_VISIBLE`, NSView, X11 | **Skia via `i-slint-renderer-skia`**: D3D on Windows, Metal on macOS ≥13 (else OpenGL), default on Linux | `request_redraw()` only sets a flag. Each frame tick runs queued callbacks, polls events, `update_timers_and_animations()`, and calls `render()` only if the flag is set. Ticks: Win = DXGI vblank thread (fallback DwmFlush, then 10 ms sleep); mac = CADisplayLink (≥14) or 16 ms NSTimer; Linux = host timer at 16 ms (CLAP `timer_support` / VST3 `IRunLoop`) | [github.com/ilmai/plugin-things](https://github.com/ilmai/plugin-things) |
| **slint-baseview** (Aidan Martins, ISC) | Generic Slint-in-baseview layer | None (framework-agnostic) | baseview `open_parented` child window | **FemtoVG over OpenGL** (`renderer-femtovg`, pinned `=1.16.1`) | baseview `on_frame` every 15 ms. It calls `request_redraw()` and `render()` **every frame unconditionally**, then `swap_buffers`. GL vsync is off. README says it's developed on Windows and macOS/Linux "want testing" | [codeberg.org/RustAudio/slint-baseview](https://codeberg.org/RustAudio/slint-baseview) |
| **nice-plug-slint** (ISC) | nice-plug (NIH-plug fork) editor adapter | nice-plug (VST3/CLAP) | Via baseview (same design as slint-baseview, which it was extracted from) | FemtoVG over OpenGL (`=1.16.1`) | Same per-frame loop. Plugin→UI values are pushed every frame in `with_event_loop` | [github.com/aidan729/nice-plug-slint](https://github.com/aidan729/nice-plug-slint), [upstream PR #90](https://codeberg.org/RustAudio/nice-plug/pulls/90) |
| **Accidental Synthesizer (AccSyn)** | Standalone macOS synth (not a plugin), Apache-2.0 | None (CoreAudio + midir) | Own top-level window via the stock backend | `slint = "1.18.0"` with **default features**, so winit + FemtoVG/OpenGL (software renderer is the fallback) | Slint's own event loop (`.run()`). Audio/MIDI threads push to the UI with `upgrade_in_event_loop` | [gitlab.com/joltedbot-public/accidental-synth](https://gitlab.com/joltedbot-public/accidental-synth) |
| **Zeedle** | Standalone music player, GPL-3.0 | None (rodio) | Stock winit window | `backend-winit` + **`renderer-skia`** (`=1.17.1`) | Slint event loop. `invoke_from_event_loop` from worker threads; `slint::Timer`s for progress (200 ms), spectrum (100 ms) and theme (1 s) | [github.com/Jordan-Haidee/Zeedle](https://github.com/Jordan-Haidee/Zeedle) |
| **Chiptrack** | Standalone Game Boy-sound sequencer/synth. Source MIT, binaries GPL-3.0 | None (cpal + midir) | Stock winit window (desktop and web). GBA build has its own platform | Desktop: `backend-winit` + `renderer-winit-femtovg` (old alias of `renderer-femtovg`). GBA: `MinimalSoftwareWindow` software renderer | Slint event loop; the sound engine pushes channel state into Slint globals | [github.com/jturcotte/chiptrack](https://github.com/jturcotte/chiptrack) |

Other reference: **Slint's official C++ `platform_native` example** ([windowadapter_win.h](https://github.com/slint-ui/slint/blob/master/examples/cpp/platform_native/windowadapter_win.h)). It creates a `WS_CHILDWINDOW` under a parent HWND and uses `slint::platform::SkiaRenderer` with `NativeWindowHandle::from_win32`. `request_redraw()` is `InvalidateRect`, and on `WM_PAINT` it calls `update_timers_and_animations()` then `render()`, re-invalidating while animations run. Its README says the interface "could even be in a plugin". In #6248 a maintainer says similar macOS code exists but wasn't published.

### Evidence (file:line)

- **plugin-things** @ `d0ec6f5`
  - `Cargo.toml:25-32`: `slint ~1.17.1` (default-features off; `accessibility`, `compat-1-2`, `std`) plus the internal crates `i-slint-core`, `i-slint-common` and `i-slint-renderer-skia ~1.17.1 (features=["x11"])`. A comment there says internal crates must be pinned because they're not semver-stable.
  - `plugin-canvas-slint/src/window_adapter.rs:68-83`: renderer choice. `SkiaRenderer::default` (Linux), `default_metal` if macOS ≥13 else `default_opengl`, `default_direct3d` (Windows). `set_window_handle(...)` points it at the plugin-canvas window.
  - `window_adapter.rs:157-177`: Draw handler. Drains the `invoke_from_event_loop` queue, `poll_events()`, `update_timers_and_animations()`, and `render()` only if `pending_draw`.
  - `window_adapter.rs:385-387`: `request_redraw()` sets `pending_draw`.
  - `plugin-canvas-slint/src/platform.rs:10-40`: custom `Platform`. `invoke_from_event_loop` pushes into a queue drained on the next frame; `quit_event_loop` is a no-op (no Slint-owned loop).
  - `plugin-canvas-slint/src/editor.rs:95`: `slint::platform::set_platform(...)` (ok to fail once already set).
  - `plugin-canvas/src/platform/win32/window.rs:148-160`: `CreateWindowExW(..., WS_CHILD | WS_VISIBLE, ..., parent)`. Lines 515-570: `frame_pacing_thread` (DXGI `IDXGIOutput::WaitForVBlank` → `DwmFlush` → 10 ms sleep, then `SendMessageW(WM_APP_FRAME_TIMER)`, which becomes `Event::Draw` at lines 467-471).
  - `plugin-canvas/src/platform/mac/window.rs:109-125`: `CADisplayLink` on macOS ≥14, else a 0.016 s `NSTimer` on the main run loop.
  - `plinth-plugin/src/editor.rs:4`: `FRAME_TIMER_MILLISECONDS = 16`. On Linux it's registered with the host: `formats/clap/extensions/gui.rs:101-105` and `formats/vst3/view.rs:200-212`.
  - HiDPI. `window_adapter.rs:55-61, 127-137, 363-383`: the plugin "scale" comes from the host's set_scale; macOS multiplies in `NSScreen.backingScaleFactor`; Slint gets `WindowEvent::ScaleFactorChanged`. `plugin-canvas/src/screen.rs`: Windows default = `LOGPIXELSX/96`, Linux = X11 physical DPI/96. `examples/gain-plugin/src/editor.rs:41-56`: the editor resizes the window to default size × scale.
  - `examples/gain-plugin/src/view.rs:55-67`: the UI pulls parameter values on every `Event::Draw`.
- **slint-baseview** @ `9e86cbc`
  - `Cargo.toml:15-22`: `baseview{opengl}`, `slint =1.16.1` with `renderer-femtovg`, `raw-window-handle-06`, `compat-1-2`.
  - `src/lib.rs:68-82`: `GlConfig{ vsync: false, double_buffer: true, srgb: true }`.
  - `src/lib.rs:406-457`: `on_frame` → update closure → `update_timers_and_animations()` → `request_redraw()` → `renderer.render()` → `swap_buffers()` every frame.
  - baseview frame timer @ `712ebfe`: `src/platform/win/window.rs:284-289` (`set_timer(WIN_FRAME_TIMER, 15)` with a FIXME saying it "should be replaced by proper window redrawing/damage/vsync handling"), `src/platform/macos/view.rs:143` (0.015 s), `src/platform/x11/event_loop.rs:126-150` (15 ms).
- **nice-plug-slint** @ `6f0828d`: `Cargo.toml:12-14`; `docs/ARCHITECTURE.md` explains the global `set_platform`-once and thread-local adapter trick, plus the lazy FemtoVG init once the GL context is current.
- **AccSyn** @ `ce08f23`: `Cargo.toml:37` `slint = "1.18.0"` (default features). Slint's default features are `backend-default`, `renderer-femtovg`, `renderer-software`, … ([slint Cargo.toml](https://github.com/slint-ui/slint/blob/master/api/rs/slint/Cargo.toml)). `crates/accidental-synth/src/ui.rs:161`: `upgrade_in_event_loop`.
- **Zeedle** @ `5c812b5`: `Cargo.toml:24-31`; `src/main.rs:1112-1211` (timers); `src/spectrum.rs:19` (`SPECTRUM_UPDATE_MS = 100`).
- **Chiptrack** @ `3cb0caa`: `Cargo.toml:28-59`; `src/gba_platform.rs:24,125` (`MinimalSoftwareWindow`).

**Performance notes found.** No plugin-specific performance issues are documented in these repos. Related Slint threads:

- [#4691](https://github.com/slint-ui/slint/discussions/4691): a maintainer attributes high CPU on desktop to "full window repaints" and says Slint doesn't yet do partial rendering on desktop GPU backends.
- [#5677](https://github.com/slint-ui/slint/discussions/5677): switching FemtoVG→Skia cut CPU from about 30% to 2%, at the cost of a bigger binary (about 3→20 MB) and more RAM (20→60 MB).
- Aava's author lists the main pains as the DSL/native boundary boilerplate and compile times, and wishes for custom shaders "without completely custom OpenGL surfaces".

## Other showcase entries (not audio)

Many showcase entries have no public source: OTIV, SK Signet, Clockworks (proprietary), plus WesAudio and Aava (above). The ones with public source are standard desktop apps (winit backend; renderer per their own features), libraries, or embedded/MCU projects (software renderer). None of them embed into a foreign window.

| Project | What | Licence | Repo |
|---|---|---|---|
| WSL Dashboard | WSL manager | GPL-3.0 | https://github.com/owu/wsl-dashboard |
| GuaViewer | Market viewer | not determined | https://github.com/azurewood/guaViewer |
| Koharu | LLM manga translation | Apache-2.0 | https://github.com/mayocream/koharu |
| Raccoin | Crypto portfolio/tax | not determined | https://git.sr.ht/~thorbjorn/raccoin |
| Fractal Explorer | Mandelbrot/Julia (web) | MIT | https://github.com/boclair/fractal-explorer |
| SurrealismUI | Component library | MIT | https://github.com/syf20020816/SurrealismUI |
| snake-game | Game | MIT | https://github.com/timsaya/snake-game |
| snake (Szybet) | Multi-platform game | not determined | https://github.com/Szybet/snake-slint |
| tymoz | Time zones | MIT | https://github.com/arunpkio/tymoz |
| InkWatchy GUI | Watch firmware UI (MCU) | GPL-3.0 | https://github.com/Szybet/InkWatchy |
| SAST Evento | Event client | MIT | https://github.com/NJUPT-SAST/sast-evento |
| coop_local / coop | File manager / widget lib | MIT (README badge) | https://codeberg.org/flovansl/co_sl |
| Dispute | Pomodoro | GPL-3.0 | https://github.com/Vinegret43/dispute |
| Tomotroid | Pomodoro | MIT | https://github.com/vadoola/Tomotroid |
| GPCL | Gamepad launcher | MIT | https://github.com/dngulin/gpcl |
| Cargo UI | Cargo GUI | Apache-2.0 | https://github.com/slint-ui/cargo-ui |
| Tetris | Game (web) | MIT | https://github.com/GaspardCulis/slint-tetris |
| Mastermind | Game | not determined | https://github.com/ElevenJune/mastermind_Rust |
| Parch Welcome | Distro welcome app | NOASSERTION per GitHub | https://github.com/parchlinux/parch-welcome |
| ImageSieve | Photo sorter | GPL-3.0 | https://github.com/Futsch1/image-sieve |
| POVVER | ABM simulator | GPL-3.0 | https://github.com/burumdev/POVVER |
| Flectar Mail | Email client | AGPL-3.0 | https://github.com/flectar/mail |
| Market Prices | Crypto quotes | not determined | https://github.com/raykavin/market-prices |
| BSP-STM32F429-DK | MCU BSP + Slint (software renderer) | MPL-2.0 | https://github.com/ierturk/rust-on-stm32 |
| Castle of Focus / vivi | Focus app / component lib | MIT (vivi README badge) | https://codeberg.org/vivi-ui/vivi |
| Oxide Manager | Game mod manager | GPL-3.0 | https://github.com/pezfisk/OxideManager |
| Ordinary | e-paper compositor/shell | GPL-3.0 | https://gitlab.com/floers/ordinary |
| musi lili | Retro game engine | LGPL-3.0-only (README badge) | https://codeberg.org/vivi-ui/lili |
| Game Bub | FPGA handheld | CERN-OHL-S-2.0 | https://github.com/elipsitz/gamebub |
| WinAlpha | Window transparency tool | not determined | https://github.com/tadghh/transparent-windows |
| Minesweeper | Game (wasm) | GPL-3.0 | https://github.com/Erik7354/slint_minesweeper |
| WebSocket Reflector X | Tunnel tool | MIT | https://github.com/XDSEC/WebSocketReflectorX |
| Project Trains Launcher | Game launcher | not determined | https://github.com/Project-Trains/launcher |

I took licences from GitHub's detected SPDX id or the repo README/LICENSE. I didn't inspect the renderer of each non-audio app.

## What this means for the JUCE + Slint PoC

1. **Why your PoC is probably slower than JUCE.** Rendering the whole UI on the CPU into a `juce::Image`, then having JUCE draw that image, costs several things that no showcase plugin pays:
   - **Rasterization on the CPU.** Slint's software renderer rasterizes every pixel itself, and at 2× HiDPI that's 4× the pixels.
   - **Possible format conversion and copy into the image.**
   - **A second blit by JUCE's own renderer.**
   - **Possibly a full-window repaint on every timer tick.**

   Aava (most likely) and Slint's own native example avoid all of this by letting **Skia on the GPU draw straight into a native child surface**. They only redraw when Slint flags it and pace frames to vsync.
2. **Recommended route: a GPU child window inside the JUCE editor, the plugin-canvas-slint pattern.**
   - **Option A, fastest to try.** Keep JUCE for audio, params and the editor shell, and call into Rust. Use `plugin-canvas` + `plugin-canvas-slint` directly: pass the editor's native handle as the parent `RawWindowHandle` to `SlintEditor::open`. That's `getPeer()->getNativeHandle()` (HWND / NSView*), or a `juce::HWNDComponent` / `NSViewComponent` / `XEmbedComponent` you create. It makes its own `WS_CHILD`/NSView and does D3D/Metal Skia rendering plus vsync pacing on Windows and macOS. On Linux, call `EditorHandle::on_frame()` from a 16 ms `juce::Timer`, because plinth normally gets that from the host timer. Forward JUCE's scale changes to `set_scale()` / `set_window_size()`.
     - Caveats: it depends on `i-slint-*` internal crates pinned to `~1.17.1` (expect breakage on Slint upgrades), Skia makes the binary much bigger, and it uses one global Slint platform per process. That's fine for multiple editors on the main thread, and it's what plugin-canvas-slint does.
   - **Option B, own the adapter.** Write a ~300-line `WindowAdapter`, modelled on `plugin-canvas-slint/src/window_adapter.rs` or Slint's C++ `platform_native/windowadapter_win.h`:
     - Create a child native view in the JUCE editor.
     - Use `SkiaRenderer` with `default_direct3d` / `default_metal`.
     - Translate JUCE mouse and key events into `WindowEvent`s.
     - Drive it from `juce::VBlankAttachment` (JUCE 7+, vsync-synced callback on the message thread): run `invoke_from_event_loop` callbacks → `update_timers_and_animations()` → `render()` only when `request_redraw` set a flag.

     Slint also has a public C++ `slint::platform::SkiaRenderer`, so this could live entirely in C++ if that's easier than FFI.
   - **Avoid slint-baseview / nice-plug-slint as a model for performance.** They render unconditionally every 15 ms on OpenGL with vsync off. OpenGL is deprecated on macOS and FemtoVG was much slower than Skia in #5677.
3. **If you must keep the software renderer** (simplest integration, no GPU interop), fix these first:
   - Use `MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer)` and keep **one persistent** pixel buffer / `juce::Image` instead of a new one per frame. Then only the dirty region gets re-rasterized: `SoftwareRenderer::render()` returns the `PhysicalRegion` it actually painted.
   - Call `draw_if_needed()` from a `VBlankAttachment`/`Timer` after `update_timers_and_animations()`. If it returns false, do nothing. If it returns true, call `repaint(dirtyBounds)` with **only the returned region** (scaled to logical coordinates), not the whole editor.
   - Render **directly into the JUCE image memory**. `render(&mut [impl TargetPixel], stride)` accepts any pixel type implementing the public `TargetPixel` trait, so you can implement it for JUCE's premultiplied BGRA `PixelARGB` layout and skip a conversion pass. Check whether a `SoftwareImageType` image is cheaper than `NativeImageType` for per-frame CPU writes on your JUCE version and backend. I didn't verify this for JUCE 8's Direct2D path.
   - Size the buffer in **physical pixels** and pass the matching scale factor to Slint (`WindowEvent::ScaleFactorChanged`), so JUCE never resamples the image.
   - Expect the software renderer to stay expensive for continuously animating areas (meters, waveforms, spectrum) and large HiDPI editors. That's when to switch to Skia GPU.
4. **Data flow, copying what the shipping code does:**
   - Push parameter/meter values from the audio side via atomics or lock-free FIFOs, and apply them to Slint properties once per frame tick, like the gain-plugin's `Event::Draw` handler. Don't apply them per audio callback.
   - Wrap host-param edits in begin/set/end gestures from Slint callbacks.
   - Never block the message thread (WesAudio's stated #1 concern).
5. **Next step.** Benchmark the PoC with Slint's Skia renderer in a child window against the software→`juce::Image` path, on the same UI at 1× and 2×, with and without a continuously animating meter. Expect the biggest gap on the animated, HiDPI case.

### Couldn't be determined

- WesAudio's and Aava's actual code: both are closed source. Aava ⇄ plugin-things is an inference from shared authorship.
- WesAudio's language (C++ or Rust) and its renderer.
- Licences marked "not determined" above.

### Commits inspected

plugin-things `d0ec6f506c2c5f7f570f095469a9d6d76df8d858` (2026-09-26) · slint-baseview `9e86cbc8428e99a32e01c82f2b63c4fa5c74d455` · nice-plug-slint `6f0828d4238b440012457f781a2d1cae683e047f` · baseview `712ebfeac66e7e6d810ea0b9f1568ddc06111c5d` · accidental-synth `ce08f239f98985b75b15a7e2fb7b5cdc21be3b43` · Zeedle `5c812b5dacfa087726317029ff228c3ea9b1298c` · chiptrack `3cb0caa5bbc23d0579cdad8187c4371bdf0723a3`.
