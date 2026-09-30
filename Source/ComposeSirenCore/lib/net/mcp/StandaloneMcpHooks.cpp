#include <juce_audio_plugin_client/detail/juce_IncludeModuleHeaders.h>
#include <juce_audio_devices/juce_audio_devices.h>
#include <juce_gui_extra/juce_gui_extra.h>
#include <juce_audio_utils/juce_audio_utils.h>
#include <juce_audio_plugin_client/Standalone/juce_StandaloneFilterWindow.h>

#include "McpDeviceHooks.h"

namespace {

juce::var failure(const juce::String& message)
{
    auto* object = new juce::DynamicObject();
    object->setProperty("ok", false);
    object->setProperty("error", message);
    return juce::var(object);
}

juce::StandalonePluginHolder* holder()
{
    return juce::StandalonePluginHolder::getInstance();
}

juce::var listAudio()
{
    auto* plugin = holder();
    if (plugin == nullptr)
        return failure("the standalone audio device is not ready");

    auto& devices = plugin->deviceManager;
    const auto setup = devices.getAudioDeviceSetup();

    juce::Array<juce::var> inputs;
    juce::Array<juce::var> outputs;
    for (auto* type : devices.getAvailableDeviceTypes()) {
        type->scanForDevices();
        const auto typeName = type->getTypeName();
        for (const auto wantInput : { true, false }) {
            for (const auto& name : type->getDeviceNames(wantInput)) {
                auto* device = new juce::DynamicObject();
                device->setProperty("type", typeName);
                device->setProperty("name", name);
                (wantInput ? inputs : outputs).add(juce::var(device));
            }
        }
    }

    auto* current = new juce::DynamicObject();
    current->setProperty("input", setup.inputDeviceName);
    current->setProperty("output", setup.outputDeviceName);
    current->setProperty("sampleRate", setup.sampleRate);
    current->setProperty("bufferSize", setup.bufferSize);

    auto* root = new juce::DynamicObject();
    root->setProperty("ok", true);
    root->setProperty("current", juce::var(current));
    root->setProperty("inputs", inputs);
    root->setProperty("outputs", outputs);
    return juce::var(root);
}

juce::var setAudio(const juce::var& request)
{
    auto* plugin = holder();
    if (plugin == nullptr)
        return failure("the standalone audio device is not ready");

    auto setup = plugin->deviceManager.getAudioDeviceSetup();
    const auto present = [&request](const char* key) {
        return request.hasProperty(key) && !request.getProperty(key, {}).isVoid();
    };
    if (present("output"))
        setup.outputDeviceName = request.getProperty("output", {}).toString();
    if (present("input"))
        setup.inputDeviceName = request.getProperty("input", {}).toString();
    if (present("sampleRate"))
        setup.sampleRate = static_cast<double>(request.getProperty("sampleRate", setup.sampleRate));
    if (present("bufferSize"))
        setup.bufferSize = static_cast<int>(request.getProperty("bufferSize", setup.bufferSize));

    const auto error = plugin->deviceManager.setAudioDeviceSetup(setup, true);
    if (error.isNotEmpty())
        return failure(error);

    auto* root = new juce::DynamicObject();
    root->setProperty("ok", true);
    root->setProperty("restarted", true);
    return juce::var(root);
}

juce::var listMidi()
{
    auto* plugin = holder();
    if (plugin == nullptr)
        return failure("the standalone MIDI devices are not ready");

    auto& devices = plugin->deviceManager;
    juce::Array<juce::var> inputs;
    for (const auto& device : juce::MidiInput::getAvailableDevices()) {
        auto* object = new juce::DynamicObject();
        object->setProperty("name", device.name);
        object->setProperty("identifier", device.identifier);
        object->setProperty("enabled", devices.isMidiInputDeviceEnabled(device.identifier));
        inputs.add(juce::var(object));
    }

    const auto selected = devices.getDefaultMidiOutputIdentifier();
    juce::Array<juce::var> outputs;
    for (const auto& device : juce::MidiOutput::getAvailableDevices()) {
        auto* object = new juce::DynamicObject();
        object->setProperty("name", device.name);
        object->setProperty("identifier", device.identifier);
        object->setProperty("selected", device.identifier == selected);
        outputs.add(juce::var(object));
    }

    auto* root = new juce::DynamicObject();
    root->setProperty("ok", true);
    root->setProperty("inputs", inputs);
    root->setProperty("outputs", outputs);
    return juce::var(root);
}

juce::var setMidi(const juce::var& request)
{
    auto* plugin = holder();
    if (plugin == nullptr)
        return failure("the standalone MIDI devices are not ready");

    const auto op = request.getProperty("op", {}).toString();
    const auto identifier = request.getProperty("identifier", {}).toString();
    if (op == "set_midi_input") {
        plugin->deviceManager.setMidiInputDeviceEnabled(
            identifier, static_cast<bool>(request.getProperty("enabled", false)));
    } else if (op == "set_midi_output") {
        plugin->deviceManager.setDefaultMidiOutputDevice(identifier);
    } else {
        return failure("unknown MIDI command");
    }

    auto* root = new juce::DynamicObject();
    root->setProperty("ok", true);
    return juce::var(root);
}

struct InstallStandaloneMcpHooks
{
    InstallStandaloneMcpHooks()
    {
        McpDeviceHooks hooks;
        hooks.listAudio = listAudio;
        hooks.setAudio = setAudio;
        hooks.listMidi = listMidi;
        hooks.setMidi = setMidi;
        McpDeviceHooks::install(std::move(hooks));
    }
};

const InstallStandaloneMcpHooks installStandaloneMcpHooks;

} // namespace
