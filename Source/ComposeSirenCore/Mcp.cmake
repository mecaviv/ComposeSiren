# composesiren-mcp: in-process MCP server, linked into every plugin format.
# The Standalone format adds device hooks (lib/net/mcp/StandaloneMcpHooks.cpp);
# the server itself is this staticlib. The C++ wrapper is lib/net/mcp/McpControl.

include("${CMAKE_SOURCE_DIR}/cmake/RustStaticLib.cmake")

# Controller reset is always included; other tools follow their feature options.
set(_cs_mcp_features reset-all-controllers)
if(COMPOSESIREN_RECORD)
  list(APPEND _cs_mcp_features record)
endif()
if(COMPOSESIREN_SONG_TITLE)
  list(APPEND _cs_mcp_features song-title)
endif()

composesiren_add_rust_staticlib(
  TARGET composesiren_mcp
  CRATE_DIR "${CMAKE_SOURCE_DIR}/Source/composesiren-mcp"
  FEATURES ${_cs_mcp_features}
  LINK_FRAMEWORKS CoreFoundation Security SystemConfiguration
)
