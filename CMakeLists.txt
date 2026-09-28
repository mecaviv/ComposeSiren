cmake_minimum_required(VERSION 3.22.0)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if (APPLE)
  set(CMAKE_OSX_ARCHITECTURES "x86_64;arm64" CACHE INTERNAL "")
  # see : https://forum.juce.com/t/best-way-to-build-with-support-for-older-macos-versions/64879/3
  set(CMAKE_OSX_DEPLOYMENT_TARGET "10.13" CACHE STRING "minimum supported target macos version")
endif()
# in windows, specifying target archs can be done by creating dedicated CMake profiles named
# "Debug/Release amd64/arm64" and associating them the corresponding visual studio toolchains

if (LINUX)
  # to fix :
  #   relocation R_X86_64_PC32 against symbol
  #   `_ZN10BinaryData18knobbackground_svgE'
  #   can not be used when making a shared object; recompile with -fPIC
  #   https://forum.juce.com/t/linux-cmake-and-fpic/54149/2
  set(CMAKE_POSITION_INDEPENDENT_CODE ON)
endif()

# Get the git hash
set(GIT_HASH "Unknown")
execute_process(
        COMMAND git rev-parse --short HEAD
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
        OUTPUT_VARIABLE _git_hash_out
        OUTPUT_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE _git_hash_rc
)

if(_git_hash_rc EQUAL 0 AND NOT "${_git_hash_out}" STREQUAL "")
  set(GIT_HASH "${_git_hash_out}")
endif()

string(TIMESTAMP CURRENT_DATE "%Y-%m-%d")
set(PROJECT_DESCRIPTION "ComposeSiren du ${CURRENT_DATE} sur commit ${GIT_HASH}")

project(ComposeSiren VERSION 2.0.0 DESCRIPTION "${PROJECT_DESCRIPTION}")
set(PROJECT_VERSION_SUFFIX "-beta1") # just for building/packaging purpose
set(BaseTargetName ${PROJECT_NAME})
set(VendorName "Mecanique Vivante")
set(PLUGIN_MANUFACTURER_CODE "McVv")

# DEAL WITH PLUGIN RESOURCES PROCESSING ########################################

option(PROCESS_RESOURCES
        "Use ResourcesProcessing outputs instead of original Resources"
        ON)

set(ORIGINAL_RESOURCES_DIR
        "${CMAKE_SOURCE_DIR}/Resources"
        CACHE PATH "Original resources directory")

set(PROCESSED_RESOURCES_DIR
        "${CMAKE_BINARY_DIR}/Resources-processed"
        CACHE PATH "Processed resources output directory")

if(PROCESS_RESOURCES)
  set(PLUGIN_RESOURCES_DIR
          "${PROCESSED_RESOURCES_DIR}"
          CACHE PATH "Resources directory for packaging" FORCE)
else()
  set(PLUGIN_RESOURCES_DIR
          "${ORIGINAL_RESOURCES_DIR}"
          CACHE PATH "Resources directory for packaging" FORCE)
endif()

# DEAL WITH THE PARK BRIDGE ####################################################
# COMPOSESIREN_PARK_BRIDGE compiles the UDP park bridge at all. Off: neither
# SirenLink nor the Rust crate is built, and the "Sirenes physiques" / "ST"
# controls are omitted. On: SirenOrchestra drives the physical sirens with
# SirenLink (C++, ComposeSirenCore/lib/net) or, if COMPOSESIREN_MECAVIV_BRIDGE
# is also ON, with mecaviv-bridge-composesiren (Rust). See
# Source/ComposeSirenCore/MecavivBridge.cmake.

option(COMPOSESIREN_PARK_BRIDGE
        "Compile the park UDP bridge (C++ SirenLink or Rust mecaviv-bridge) and show Sirenes physiques / ST"
        OFF)

option(COMPOSESIREN_MECAVIV_BRIDGE
        "Drive the park with mecaviv-bridge-composesiren (Rust) instead of SirenLink (requires COMPOSESIREN_PARK_BRIDGE)"
        OFF)

# DEAL WITH THE MCP SERVER #####################################################
# Each plugin instance can run an in-process MCP server (Rust crate
# Source/composesiren-mcp). It does not use mecaviv-rs. See
# Source/ComposeSirenCore/Mcp.cmake.

option(COMPOSESIREN_MCP
        "In-process MCP server for parameters, MIDI, and standalone audio/MIDI devices"
        ON)

#set(PLUGIN_TARGETS OneSiren)
set(PLUGIN_TARGETS OneSiren SirenOrchestra)
#set(PLUGIN_TARGETS CustomMix)

add_subdirectory(Dependencies/JUCE)

set(FORMATS "VST3" "Standalone")

if (VST2_PATH)
  list(APPEND FORMATS "VST")
  juce_set_vst2_sdk_path(${VST2_PATH})
endif()

if (AAX_PATH)
  list(APPEND FORMATS "AAX")
  juce_set_aax_sdk_path(${AAX_PATH})
endif()

if (APPLE)
  list(APPEND FORMATS "AU")
endif()

add_subdirectory(Assets)
add_subdirectory(Source)

if(PROCESS_RESOURCES)
  add_subdirectory(ResourcesProcessing)
endif()

if(APPLE) # we generate a cmake subproject to build the installer
  include(Packaging/Apple/MakePackage.cmake)
elseif(WIN32) # we just proceed the regular way
  include(Packaging/Windows/MakePackage.cmake)
  #include(Packaging/Wix/MakePackage.cmake)
endif()
