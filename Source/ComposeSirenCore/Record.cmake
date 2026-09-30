# composesiren-record: the recorder (FLAC or WAV of the audio output), pure
# Rust, linked into ComposeSirenCore when COMPOSESIREN_RECORD is ON. The C
# header is under the crate's include/ directory; the C++ wrapper is
# ComposeSirenCore/Recorder.

set(COMPOSESIREN_RECORD_CRATE_DIR "${CMAKE_SOURCE_DIR}/Source/composesiren-record")
set(COMPOSESIREN_RECORD_INCLUDE_DIR "${COMPOSESIREN_RECORD_CRATE_DIR}/include")

set(_cs_rec_cargo_env "")
if(APPLE AND CMAKE_OSX_DEPLOYMENT_TARGET)
  list(APPEND _cs_rec_cargo_env "MACOSX_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}")
endif()

list(LENGTH CMAKE_OSX_ARCHITECTURES _cs_rec_arch_count)

if(APPLE AND _cs_rec_arch_count GREATER 1)
  find_program(COMPOSESIREN_RECORD_CARGO cargo
    HINTS "$ENV{CARGO_HOME}/bin" "$ENV{HOME}/.cargo/bin"
    REQUIRED)
  find_program(COMPOSESIREN_RECORD_RUSTUP rustup
    HINTS "$ENV{CARGO_HOME}/bin" "$ENV{HOME}/.cargo/bin")
  find_program(COMPOSESIREN_RECORD_LIPO lipo REQUIRED)

  if(COMPOSESIREN_RECORD_RUSTUP)
    execute_process(
      COMMAND "${COMPOSESIREN_RECORD_RUSTUP}" target list --installed
      OUTPUT_VARIABLE _cs_rec_installed_targets
      OUTPUT_STRIP_TRAILING_WHITESPACE)
  endif()

  set(_cs_rec_target_dir "${CMAKE_CURRENT_BINARY_DIR}/composesiren-record-target")
  set(_cs_rec_profile "$<IF:$<CONFIG:Debug>,dev,release>")
  set(_cs_rec_profile_dir "$<IF:$<CONFIG:Debug>,debug,release>")
  set(_cs_rec_lib_name "libcomposesiren_record.a")
  set(_cs_rec_lib "${CMAKE_CURRENT_BINARY_DIR}/composesiren-record/$<CONFIG>/${_cs_rec_lib_name}")

  set(_cs_rec_target_flags "")
  set(_cs_rec_arch_libs "")
  foreach(_arch IN LISTS CMAKE_OSX_ARCHITECTURES)
    if(_arch STREQUAL "arm64")
      set(_triple "aarch64-apple-darwin")
    elseif(_arch STREQUAL "x86_64")
      set(_triple "x86_64-apple-darwin")
    else()
      message(FATAL_ERROR "composesiren-record: no Rust target for macOS architecture ${_arch}")
    endif()
    if(COMPOSESIREN_RECORD_RUSTUP AND NOT _cs_rec_installed_targets MATCHES "(^|\n)${_triple}(\n|$)")
      message(FATAL_ERROR
        "composesiren-record: the universal build needs the Rust target ${_triple}. "
        "Install it with: rustup target add ${_triple}")
    endif()
    list(APPEND _cs_rec_target_flags "--target=${_triple}")
    list(APPEND _cs_rec_arch_libs
      "${_cs_rec_target_dir}/${_triple}/${_cs_rec_profile_dir}/${_cs_rec_lib_name}")
  endforeach()

  add_custom_target(composesiren_record_universal
    COMMAND "${CMAKE_COMMAND}" -E env
      ${_cs_rec_cargo_env}
      "CARGO_TARGET_DIR=${_cs_rec_target_dir}"
      "${COMPOSESIREN_RECORD_CARGO}" build
        --manifest-path "${COMPOSESIREN_RECORD_CRATE_DIR}/Cargo.toml"
        --lib
        "--profile=${_cs_rec_profile}"
        ${_cs_rec_target_flags}
    COMMAND "${CMAKE_COMMAND}" -E make_directory
      "${CMAKE_CURRENT_BINARY_DIR}/composesiren-record/$<CONFIG>"
    COMMAND "${COMPOSESIREN_RECORD_LIPO}" -create ${_cs_rec_arch_libs} -output "${_cs_rec_lib}"
    BYPRODUCTS "${_cs_rec_lib}"
    COMMENT "Building composesiren-record for ${CMAKE_OSX_ARCHITECTURES}"
    VERBATIM
    USES_TERMINAL
  )

  add_library(composesiren_record INTERFACE)
  target_link_libraries(composesiren_record INTERFACE "${_cs_rec_lib}")
  add_dependencies(composesiren_record composesiren_record_universal)
else()
  include(FetchContent)
  FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6.1
  )
  FetchContent_MakeAvailable(Corrosion)

  corrosion_import_crate(
    MANIFEST_PATH "${COMPOSESIREN_RECORD_CRATE_DIR}/Cargo.toml"
    CRATE_TYPES staticlib
  )
  if(_cs_rec_cargo_env)
    corrosion_set_env_vars(composesiren_record ${_cs_rec_cargo_env})
  endif()
endif()

target_include_directories(composesiren_record INTERFACE "${COMPOSESIREN_RECORD_INCLUDE_DIR}")
