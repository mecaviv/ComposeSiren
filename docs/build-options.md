# Build options

Every switch that changes what ComposeSiren compiles, from `CMakeLists.txt`, `Config.cmake`,
`cmake/*.cmake` and `Source/**/CMakeLists.txt`. The `composesiren_option` ones are also recorded in the
build info that the About dialog shows (`cmake/BuildInfo.cmake`).

## CMake options (`-D<NAME>=ON|OFF`)

| Option | Default | C++ define | What it does | Needs |
|---|---|---|---|---|
| `COMPOSESIREN_CLIC` | OFF | `COMPOSESIREN_CLIC=0/1` | Click output. SirenOrchestra gets a second stereo bus, "Clic", played by the click box's engine (Rust crate `Source/clic-composesiren`). MIDI channel 10: note on with note > 1 and velocity > 1 (even note = strong click, odd note = weak click); program change picks the click. Adds the Clic pane (enable switch, Spread, Decay, Bias, Vol) to the SirenOrchestra editor. Also compiles the settings store, because the clic's secondary output device is a setting. | `firmwares-artila` (with `ClicRaspberry/clic-core`) checked out next to ComposeSiren |
| `COMPOSESIREN_PARK_BRIDGE` | OFF | `COMPOSESIREN_PARK_BRIDGE=0/1` | The UDP bridge to the physical sirens of the park. On: the "Sirenes physiques" and "ST" toggles in SirenOrchestra's top bar, and one ST LED per track (state read from the KEB drive). SirenLink (C++, `lib/net`) unless `COMPOSESIREN_MECAVIV_BRIDGE` is on. | |
| `COMPOSESIREN_MECAVIV_BRIDGE` | OFF | `COMPOSESIREN_MECAVIV_BRIDGE=1` (interface define of the Rust target) | Drives the park with the Rust `mecaviv-bridge-composesiren` crate instead of SirenLink. Ignored unless `COMPOSESIREN_PARK_BRIDGE` is on. | `COMPOSESIREN_PARK_BRIDGE`; `mecaviv-rs` checked out next to ComposeSiren |
| `COMPOSESIREN_MCP` | **ON** | `COMPOSESIREN_MCP=0/1` | An in-process MCP server per plugin instance (Rust crate `Source/composesiren-mcp`): parameters, MIDI, and the Standalone's audio/MIDI devices (`StandaloneMcpHooks.cpp` is added to the Standalone targets). | |
| `COMPOSESIREN_RECORD` | OFF | `COMPOSESIREN_RECORD=0/1` | Records the audio output to FLAC or WAV (Rust crate `Source/composesiren-record`): Record... in the Menu, and the `start_recording` / `stop_recording` / `recording_status` MCP tools (cargo feature `record` of the MCP crate). | MCP tools only with `COMPOSESIREN_MCP` |
| `COMPOSESIREN_SETTINGS` | **ON** | `COMPOSESIREN_SETTINGS=0/1` | Power-user settings described by `lib/definitions/generated/SettingsMetadata.h`, stored per user, edited in Settings... (in the Menu). Off: every setting keeps its default. | |
| `COMPOSESIREN_SONG_TITLE` | OFF | `COMPOSESIREN_SONG_TITLE=0/1` | Shows the playing song on the left of the top bar and its progress in the window title, set over MCP (`set_song_title`, `set_song_progress`, `clear_song_title`; cargo feature `song-title` of the MCP crate). | `COMPOSESIREN_MCP` for the tools |
| `COMPOSESIREN_SLINT_UI` | OFF | `COMPOSESIREN_SLINT_UI=0/1` (per plugin target) | Experiment: the editors drawn by the Slint UI written in Rust (`Source/composesiren-slint-ui`), rendered into the JUCE editor's own window. JUCE keeps audio, parameters, formats and hosting. Off: the JUCE editors, unchanged. See [slint-port-investigation.md](slint-port-investigation.md). | Rust 1.92 or newer |
| `PROCESS_RESOURCES` | **ON** | (none) | Uses the `ResourcesProcessing` outputs (`<build>/Resources-processed`) instead of the original `Resources` for the siren samples. | Python with `ResourcesProcessing/requirements.txt` |
| `MACOS_UNIVERSAL` | OFF | (none) | macOS only: fat x86_64 + arm64 binaries (the Rust crates are built per architecture and joined with `lipo`). Off: the host architecture only. | the second Rust target (`rustup target add`), added automatically when rustup is there |
| `COMPOSESIREN_GIT_SUBMODULE` | **ON** | (none) | Initialise missing git submodules (`Dependencies/JUCE`) at configure time. | |

Debug builds are where the development defines differ: `COMPOSESIREN_DEV_BUILD=1` and
`COMPOSESIREN_DEV_RESOURCES_DIR=<resources dir>` (the plugins load samples from the source tree);
any other build type gets `COMPOSESIREN_DEV_BUILD=0` and an empty resources dir.

## Cache variables and paths

| Variable | Default | What it does |
|---|---|---|
| `CMAKE_BUILD_TYPE` | (generator default) | `Debug` turns on the development defines above. |
| `VST2_PATH` | empty (`Config.cmake`) | Path to a VST2 SDK; adds the `VST` format. |
| `AAX_PATH` | unset | Path to the AAX SDK; adds the `AAX` format. |
| `ORIGINAL_RESOURCES_DIR` | `<source>/Resources` | The original samples. |
| `PROCESSED_RESOURCES_DIR` | `<build>/Resources-processed` | Where `ResourcesProcessing` writes. |
| `CMAKE_OSX_DEPLOYMENT_TARGET` | `10.13` | Minimum macOS. |
| `APPLE_DEVELOPER_ID_APPLICATION`, `APPLE_DEVELOPER_ID_INSTALLER`, `ENABLE_NOTARIZATION`, `APPLE_NOTARIZATION_KEYCHAIN_PROFILE` | empty / FALSE (`Config.cmake`) | Signing and notarisation of the macOS installer; no effect on the plugin code. |

Fixed, not switchable: `CMS_BUILD_WITH_CMAKE=1`, `JUCE_VST3_CAN_REPLACE_VST2=0`, `JUCE_WEB_BROWSER=0`,
`PROJECT_VERSION_*`, `PROJECT_DESCRIPTION`, `GIT_HASH`, and `JUCE_USE_CURL=0` on Linux. The formats are
VST3 and Standalone, plus AU on macOS. `PLUGIN_TARGETS` (OneSiren, SirenOrchestra) is set in
`CMakeLists.txt`, not on the command line.

## Everything on: CLion's cmake and Ninja, Debug, `cmake-build-debug`

The configure line that turns on every feature option. In CLion, put the `-D` options in *Settings |
Build, Execution, Deployment | CMake | CMake options* of the Debug profile (build directory
`cmake-build-debug`).

```sh
CLION=~/Applications/CLion.app/Contents/bin   # /Applications/CLion.app/... for a system-wide install; mac/aarch64 on Apple silicon
$CLION/cmake/mac/x64/bin/cmake -S . -B cmake-build-debug -G Ninja \
  -DCMAKE_BUILD_TYPE=Debug \
  -DCMAKE_MAKE_PROGRAM=$CLION/ninja/mac/x64/ninja \
  -DCOMPOSESIREN_CLIC=ON \
  -DCOMPOSESIREN_PARK_BRIDGE=ON \
  -DCOMPOSESIREN_MECAVIV_BRIDGE=ON \
  -DCOMPOSESIREN_MCP=ON \
  -DCOMPOSESIREN_RECORD=ON \
  -DCOMPOSESIREN_SETTINGS=ON \
  -DCOMPOSESIREN_SONG_TITLE=ON \
  -DCOMPOSESIREN_SLINT_UI=ON \
  -DPROCESS_RESOURCES=ON \
  -DCOMPOSESIREN_GIT_SUBMODULE=ON
$CLION/cmake/mac/x64/bin/cmake --build cmake-build-debug --target OneSiren_Standalone SirenOrchestra_Standalone
```

`MACOS_UNIVERSAL` is left OFF here on purpose: it is an architecture choice, not a feature, and it
doubles the Rust builds. Add `-DMACOS_UNIVERSAL=ON` for release packages.

For the screenshots with the click output off, a second build directory uses the same line with
`-DCOMPOSESIREN_CLIC=OFF` and `-B cmake-build-debug-noclic`.
