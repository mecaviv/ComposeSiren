# composesiren-slint-ui: the Slint editor experiment (Rust staticlib), linked
# into OneSiren when COMPOSESIREN_SLINT_UI is ON. The C header is under the
# crate's include/ directory; the JUCE editor is juce/SlintOneSirenEditor.h.
# Default features only: the staticlib renders with Slint's software renderer
# into the JUCE editor, no winit (that is the `standalone` preview feature).

include("${CMAKE_SOURCE_DIR}/cmake/RustStaticLib.cmake")

if(NOT TARGET composesiren_slint_ui)
  composesiren_add_rust_staticlib(
    TARGET composesiren_slint_ui
    CRATE_DIR "${CMAKE_SOURCE_DIR}/Source/composesiren-slint-ui"
    # font discovery (fontique) on macOS; JUCE links them too
    LINK_FRAMEWORKS CoreFoundation CoreGraphics CoreText
  )
endif()
