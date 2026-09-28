#include "McpDeviceHooks.h"

namespace {

struct Store
{
    std::mutex mutex;
    McpDeviceHooks hooks;
    bool installed = false;
};

// Function-local so a standalone static constructor can install the hooks
// without a later dynamic initializer in this file wiping them.
Store& store()
{
    static Store store;
    return store;
}

} // namespace

void McpDeviceHooks::install(McpDeviceHooks next)
{
    auto& state = store();
    std::lock_guard lock(state.mutex);
    state.hooks = std::move(next);
    state.installed = true;
}

bool McpDeviceHooks::installed()
{
    auto& state = store();
    std::lock_guard lock(state.mutex);
    return state.installed;
}

McpDeviceHooks McpDeviceHooks::copy()
{
    auto& state = store();
    std::lock_guard lock(state.mutex);
    return state.hooks;
}
