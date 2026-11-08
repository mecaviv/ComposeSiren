# composesiren-slint-ui: the Slint editor experiment (Rust staticlib), linked
# into OneSiren when COMPOSESIREN_SLINT_UI is ON. The C header is under the
# crate's include/ directory; the JUCE editor is
# Source/Plugins/OneSiren/SlintOneSirenEditor.h.
# GPU code is linked, but only a JUCE standalone instance requests it. DAW
# plugin instances keep software rendering. No additional native window or event loop.

include("${CMAKE_SOURCE_DIR}/cmake/RustStaticLib.cmake")

if(NOT TARGET composesiren_slint_ui)
  composesiren_add_rust_staticlib(
    TARGET composesiren_slint_ui
    CRATE_DIR "${CMAKE_SOURCE_DIR}/Source/composesiren-slint-ui"
    FEATURES gpu
    # font discovery (fontique) on macOS; JUCE links them too
    LINK_FRAMEWORKS CoreFoundation CoreGraphics CoreText Metal QuartzCore Foundation IOKit
  )
endif()
