# composesiren-record: the recorder (FLAC or WAV of the audio output), pure
# Rust, linked into ComposeSirenCore when COMPOSESIREN_RECORD is ON. The C
# header is under the crate's include/ directory; the C++ wrapper is
# lib/utilities/recorder/Recorder.

include("${CMAKE_SOURCE_DIR}/cmake/RustStaticLib.cmake")

composesiren_add_rust_staticlib(
  TARGET composesiren_record
  CRATE_DIR "${CMAKE_SOURCE_DIR}/Source/composesiren-record"
)
