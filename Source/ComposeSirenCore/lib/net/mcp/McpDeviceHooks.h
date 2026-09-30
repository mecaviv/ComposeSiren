#pragma once

#include <functional>
#include <juce_core/juce_core.h>
#include <mutex>

// Device operations only exist in the standalone. StandaloneMcpHooks.cpp,
// compiled into the Standalone target, installs these. A plugin hosted in a
// DAW leaves them empty, and the MCP tools say the host owns the devices.

struct McpDeviceHooks
{
    std::function<juce::var()> listAudio;
    std::function<juce::var(const juce::var&)> setAudio;
    std::function<juce::var()> listMidi;
    std::function<juce::var(const juce::var&)> setMidi;

    static void install(McpDeviceHooks hooks);
    static bool installed();
    static McpDeviceHooks copy();
};
