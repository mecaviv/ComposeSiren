#include "McpControl.h"

#include "McpDeviceHooks.h"

#include <cstring>

namespace {

const char* hostOwnsDevices =
    "Audio and MIDI devices belong to the host. Open the standalone to change them.";

juce::var failure(const juce::String& message)
{
    auto* object = new juce::DynamicObject();
    object->setProperty("ok", false);
    object->setProperty("error", message);
    return juce::var(object);
}

juce::String jsonOf(const juce::var& value)
{
    return juce::JSON::toString(value, true);
}

juce::var okObject()
{
    auto* object = new juce::DynamicObject();
    object->setProperty("ok", true);
    return juce::var(object);
}

} // namespace

McpControl::McpControl(juce::AudioProcessor& processorIn,
                       const juce::String& pluginName,
                       const juce::String& pluginCode)
    : processor(processorIn),
      state(std::make_shared<State>())
{
    state->processor = &processor;
    state->control = this;

#if COMPOSESIREN_MCP
    const int standalone = McpDeviceHooks::installed() ? 1 : 0;
    server = cs_mcp_start(pluginName.toRawUTF8(),
                          pluginCode.toRawUTF8(),
                          standalone,
                          &McpControl::dispatch,
                          state.get(),
                          &port);
#else
    juce::ignoreUnused(pluginName, pluginCode);
#endif
}

McpControl::~McpControl()
{
    state->shuttingDown.store(true);
    pump();
    state->control = nullptr;
#if COMPOSESIREN_MCP
    cs_mcp_stop(server);
#endif
    server = nullptr;
    state->alive.store(false);
    state->processor = nullptr;
}

void McpControl::pump()
{
    if (!state->alive.load() || state->processor == nullptr)
        return;

    std::vector<std::shared_ptr<Job>> jobs;
    {
        std::lock_guard lock(state->jobsMutex);
        jobs.swap(state->jobs);
    }

    for (const auto& job : jobs) {
        juce::var parsed;
        const auto parseResult = juce::JSON::parse(juce::String(job->request), parsed);
        const auto response = parseResult.wasOk()
            ? handle(parsed)
            : jsonOf(failure("the command was not JSON")).toStdString();
        {
            std::lock_guard lock(job->mutex);
            job->response = response;
            job->done = true;
        }
        job->cv.notify_one();
    }
}

void McpControl::drainMidi(juce::MidiBuffer& midi) const
{
    std::vector<juce::MidiMessage> messages;
    {
        std::lock_guard lock(state->midiMutex);
        messages.swap(state->midi);
    }
    for (const auto& message : messages)
        midi.addEvent(message, 0);
}

char* McpControl::dispatch(const char* requestJson, void* user)
{
    auto* raw = static_cast<State*>(user);
    if (raw == nullptr || raw->shuttingDown.load() || !raw->alive.load())
        return duplicate("{\"ok\":false,\"error\":\"server is stopping\"}");

    // The state is owned by a shared_ptr in McpControl. The callback only has
    // the raw pointer; the server is stopped before that pointer is cleared.
    auto job = std::make_shared<Job>();
    job->request = requestJson != nullptr ? requestJson : "";
    {
        std::lock_guard lock(raw->jobsMutex);
        if (raw->shuttingDown.load())
            return duplicate("{\"ok\":false,\"error\":\"server is stopping\"}");
        raw->jobs.push_back(job);
    }

    auto* messageManager = juce::MessageManager::getInstanceWithoutCreating();
    if (messageManager != nullptr && messageManager->isThisTheMessageThread()) {
        if (raw->control != nullptr)
            raw->control->pump();
    } else if (messageManager != nullptr) {
        juce::MessageManager::callAsync([raw] {
            if (!raw->alive.load() || raw->control == nullptr)
                return;
            raw->control->pump();
        });
    }

    {
        std::unique_lock lock(job->mutex);
        job->cv.wait_for(lock, std::chrono::seconds(2), [&] { return job->done; });
        if (!job->done)
            return duplicate("{\"ok\":false,\"error\":\"the message thread did not answer\"}");
        return duplicate(job->response);
    }
}

char* McpControl::duplicate(const std::string& text)
{
    auto* copy = static_cast<char*>(std::malloc(text.size() + 1));
    if (copy == nullptr)
        return nullptr;
    std::memcpy(copy, text.data(), text.size() + 1);
    return copy;
}

std::string McpControl::handle(const juce::var& request) const
{
    const auto op = request.getProperty("op", {}).toString();

    if (op == "list_parameters")
        return listParameters();
    if (op == "get_parameter")
        return getParameter(request.getProperty("id", {}).toString());
    if (op == "set_parameter")
        return setParameter(request.getProperty("id", {}).toString(),
                            static_cast<double>(request.getProperty("value", 0)));
    if (op == "send_midi")
        return sendMidi(static_cast<int>(request.getProperty("status", 0)),
                        static_cast<int>(request.getProperty("data1", 0)),
                        static_cast<int>(request.getProperty("data2", 0)));

    const auto hooks = McpDeviceHooks::copy();
    if (op == "list_audio_devices")
        return jsonOf(hooks.listAudio ? hooks.listAudio() : failure(hostOwnsDevices)).toStdString();
    if (op == "set_audio_device")
        return jsonOf(hooks.setAudio ? hooks.setAudio(request) : failure(hostOwnsDevices)).toStdString();
    if (op == "list_midi_devices")
        return jsonOf(hooks.listMidi ? hooks.listMidi() : failure(hostOwnsDevices)).toStdString();
    if (op == "set_midi_input" || op == "set_midi_output")
        return jsonOf(hooks.setMidi ? hooks.setMidi(request) : failure(hostOwnsDevices)).toStdString();

    return jsonOf(failure("unknown command " + op)).toStdString();
}

juce::RangedAudioParameter* McpControl::findParameter(const juce::String& id) const
{
    juce::RangedAudioParameter* byName = nullptr;
    int nameMatches = 0;

    for (auto* parameter : processor.getParameters()) {
        auto* ranged = dynamic_cast<juce::RangedAudioParameter*>(parameter);
        if (ranged == nullptr)
            continue;
        if (ranged->getParameterID() == id)
            return ranged;
        if (ranged->getName(128).equalsIgnoreCase(id)) {
            byName = ranged;
            ++nameMatches;
        }
    }
    return nameMatches == 1 ? byName : nullptr;
}

juce::var McpControl::parameterObject(juce::RangedAudioParameter& parameter)
{
    const auto range = parameter.getNormalisableRange();
    auto* object = new juce::DynamicObject();
    object->setProperty("id", parameter.getParameterID());
    object->setProperty("name", parameter.getName(128));
    object->setProperty("value", parameter.convertFrom0to1(parameter.getValue()));
    object->setProperty("text", parameter.getCurrentValueAsText());
    object->setProperty("minimum", range.start);
    object->setProperty("maximum", range.end);
    object->setProperty("default", parameter.convertFrom0to1(parameter.getDefaultValue()));
    return juce::var(object);
}

std::string McpControl::listParameters() const
{
    juce::Array<juce::var> parameters;
    for (auto* parameter : processor.getParameters()) {
        if (auto* ranged = dynamic_cast<juce::RangedAudioParameter*>(parameter))
            parameters.add(parameterObject(*ranged));
    }
    auto* object = new juce::DynamicObject();
    object->setProperty("ok", true);
    object->setProperty("parameters", parameters);
    return jsonOf(juce::var(object)).toStdString();
}

std::string McpControl::getParameter(const juce::String& id) const
{
    auto* parameter = findParameter(id);
    if (parameter == nullptr)
        return jsonOf(failure("no parameter named " + id)).toStdString();
    auto object = parameterObject(*parameter);
    object.getDynamicObject()->setProperty("ok", true);
    return jsonOf(object).toStdString();
}

std::string McpControl::setParameter(const juce::String& id, double value) const
{
    auto* parameter = findParameter(id);
    if (parameter == nullptr)
        return jsonOf(failure("no parameter named " + id)).toStdString();

    const auto normalised = parameter->convertTo0to1(static_cast<float>(value));
    parameter->beginChangeGesture();
    parameter->setValueNotifyingHost(normalised);
    parameter->endChangeGesture();

    auto object = parameterObject(*parameter);
    object.getDynamicObject()->setProperty("ok", true);
    return jsonOf(object).toStdString();
}

std::string McpControl::sendMidi(int status, int data1, int data2) const
{
    if (status < 0 || status > 255 || data1 < 0 || data1 > 127 || data2 < 0 || data2 > 127)
        return jsonOf(failure("MIDI bytes are out of range")).toStdString();

    const unsigned char bytes[] = {
        static_cast<unsigned char>(status),
        static_cast<unsigned char>(data1),
        static_cast<unsigned char>(data2),
    };
    {
        std::lock_guard lock(state->midiMutex);
        state->midi.emplace_back(bytes, 3);
    }
    auto object = okObject();
    object.getDynamicObject()->setProperty("queued", true);
    return jsonOf(object).toStdString();
}
