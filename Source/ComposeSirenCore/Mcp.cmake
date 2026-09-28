# composesiren-mcp: in-process MCP server, linked into every plugin format.
# The Standalone format adds device hooks; the server itself is this staticlib.

set(COMPOSESIREN_MCP_CRATE_DIR "${CMAKE_SOURCE_DIR}/Source/composesiren-mcp")
set(COMPOSESIREN_MCP_INCLUDE_DIR "${COMPOSESIREN_MCP_CRATE_DIR}/include")

set(_cs_mcp_cargo_env "")
if(APPLE AND CMAKE_OSX_DEPLOYMENT_TARGET)
  list(APPEND _cs_mcp_cargo_env "MACOSX_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}")
endif()

list(LENGTH CMAKE_OSX_ARCHITECTURES _cs_mcp_arch_count)

if(APPLE AND _cs_mcp_arch_count GREATER 1)
  find_program(COMPOSESIREN_MCP_CARGO cargo
    HINTS "$ENV{CARGO_HOME}/bin" "$ENV{HOME}/.cargo/bin"
    REQUIRED)
  find_program(COMPOSESIREN_MCP_RUSTUP rustup
    HINTS "$ENV{CARGO_HOME}/bin" "$ENV{HOME}/.cargo/bin")
  find_program(COMPOSESIREN_MCP_LIPO lipo REQUIRED)

  if(COMPOSESIREN_MCP_RUSTUP)
    execute_process(
      COMMAND "${COMPOSESIREN_MCP_RUSTUP}" target list --installed
      OUTPUT_VARIABLE _cs_mcp_installed_targets
      OUTPUT_STRIP_TRAILING_WHITESPACE)
  endif()

  set(_cs_mcp_target_dir "${CMAKE_CURRENT_BINARY_DIR}/composesiren-mcp-target")
  set(_cs_mcp_profile "$<IF:$<CONFIG:Debug>,dev,release>")
  set(_cs_mcp_profile_dir "$<IF:$<CONFIG:Debug>,debug,release>")
  set(_cs_mcp_lib_name "libcomposesiren_mcp.a")
  set(_cs_mcp_lib "${CMAKE_CURRENT_BINARY_DIR}/composesiren-mcp/$<CONFIG>/${_cs_mcp_lib_name}")

  set(_cs_mcp_target_flags "")
  set(_cs_mcp_arch_libs "")
  foreach(_arch IN LISTS CMAKE_OSX_ARCHITECTURES)
    if(_arch STREQUAL "arm64")
      set(_triple "aarch64-apple-darwin")
    elseif(_arch STREQUAL "x86_64")
      set(_triple "x86_64-apple-darwin")
    else()
      message(FATAL_ERROR "composesiren-mcp: no Rust target for macOS architecture ${_arch}")
    endif()
    if(COMPOSESIREN_MCP_RUSTUP AND NOT _cs_mcp_installed_targets MATCHES "(^|\n)${_triple}(\n|$)")
      message(FATAL_ERROR
        "composesiren-mcp: the universal build needs the Rust target ${_triple}. "
        "Install it with: rustup target add ${_triple}")
    endif()
    list(APPEND _cs_mcp_target_flags "--target=${_triple}")
    list(APPEND _cs_mcp_arch_libs
      "${_cs_mcp_target_dir}/${_triple}/${_cs_mcp_profile_dir}/${_cs_mcp_lib_name}")
  endforeach()

  add_custom_target(composesiren_mcp_universal
    COMMAND "${CMAKE_COMMAND}" -E env
      ${_cs_mcp_cargo_env}
      "CARGO_TARGET_DIR=${_cs_mcp_target_dir}"
      "${COMPOSESIREN_MCP_CARGO}" build
        --manifest-path "${COMPOSESIREN_MCP_CRATE_DIR}/Cargo.toml"
        --lib
        "--profile=${_cs_mcp_profile}"
        ${_cs_mcp_target_flags}
    COMMAND "${CMAKE_COMMAND}" -E make_directory
      "${CMAKE_CURRENT_BINARY_DIR}/composesiren-mcp/$<CONFIG>"
    COMMAND "${COMPOSESIREN_MCP_LIPO}" -create ${_cs_mcp_arch_libs} -output "${_cs_mcp_lib}"
    BYPRODUCTS "${_cs_mcp_lib}"
    COMMENT "Building composesiren-mcp for ${CMAKE_OSX_ARCHITECTURES}"
    VERBATIM
    USES_TERMINAL
  )

  add_library(composesiren_mcp INTERFACE)
  target_link_libraries(composesiren_mcp INTERFACE "${_cs_mcp_lib}")
  add_dependencies(composesiren_mcp composesiren_mcp_universal)
else()
  include(FetchContent)
  FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6.1
  )
  FetchContent_MakeAvailable(Corrosion)

  corrosion_import_crate(
    MANIFEST_PATH "${COMPOSESIREN_MCP_CRATE_DIR}/Cargo.toml"
    CRATE_TYPES staticlib
  )
  if(_cs_mcp_cargo_env)
    corrosion_set_env_vars(composesiren_mcp ${_cs_mcp_cargo_env})
  endif()
endif()

target_include_directories(composesiren_mcp INTERFACE "${COMPOSESIREN_MCP_INCLUDE_DIR}")
if(APPLE)
  target_link_libraries(composesiren_mcp INTERFACE
    "-framework CoreFoundation"
    "-framework Security"
    "-framework SystemConfiguration"
  )
endif()
