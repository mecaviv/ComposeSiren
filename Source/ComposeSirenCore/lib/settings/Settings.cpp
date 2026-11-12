#include "Settings.h"

#ifndef COMPOSESIREN_RUST_DSP
#define COMPOSESIREN_RUST_DSP 0
#endif
#if COMPOSESIREN_RUST_DSP
#include "../wrappers/DspChoice.h"
#endif

#ifndef COMPOSESIREN_SIREN_WAVES
#define COMPOSESIREN_SIREN_WAVES 0
#endif
#ifndef COMPOSESIREN_DSP_CONTROL_MIDI_IN
#define COMPOSESIREN_DSP_CONTROL_MIDI_IN 0
#endif
#ifndef COMPOSESIREN_SIREN_CONTROL_MIDI_IN
#define COMPOSESIREN_SIREN_CONTROL_MIDI_IN 0
#endif

namespace cs {

namespace meta = mecaviv::metadata;

#if COMPOSESIREN_RUST_DSP
namespace {
// the choice at `index` of a Choice setting, as its id
std::string choiceAt(const meta::Setting& s, int index)
{
    std::string_view rest = s.choices;
    for (int i = 0;; ++i) {
        const auto bar = rest.find('|');
        if (i == index) return std::string(rest.substr(0, bar));
        if (bar == std::string_view::npos) return {};
        rest.remove_prefix(bar + 1);
    }
}
} // namespace

// the dsp.* settings, for the voices (DspChoice.h)
static void publishDspChoice(const Settings& s)
{
    DspChoice c;
    c.variant = choiceAt(Settings::describe(meta::SettingId::dspVariant), s.getInt(meta::SettingId::dspVariant));
    c.dataset = choiceAt(Settings::describe(meta::SettingId::dspDataset), s.getInt(meta::SettingId::dspDataset));
    c.skipSilent = s.getBool(meta::SettingId::dspSkipSilentFrames);
    setDspChoice(c);
}
#endif

Settings::Settings()
{
    juce::PropertiesFile::Options options;
    options.applicationName = "ComposeSiren";
    options.filenameSuffix = ".settings";
    options.folderName = "Mecanique Vivante";
    options.osxLibrarySubFolder = "Application Support";
    options.storageFormat = juce::PropertiesFile::storeAsXML;
    options.millisecondsBeforeSaving = 500;
    file = std::make_unique<juce::PropertiesFile>(options);

    for (std::size_t i = 0; i < settingCount; ++i) {
        const auto id = idAt(i);
        juce::var v = describe(id).type == meta::SettingType::String
                          ? juce::var(juce::String())
                          : juce::var(describe(id).defaultValue);
        if (file->containsKey(key(id).toString())) {
            if (describe(id).type == meta::SettingType::String)
                v = file->getValue(key(id).toString());
            else
                v = file->getValue(key(id).toString()).getDoubleValue();
        }
        tree.setProperty(key(id), normalised(id, v), nullptr);
    }
    tree.addListener(this);
#if COMPOSESIREN_RUST_DSP
    publishDspChoice(*this);
#endif
}

Settings::~Settings()
{
    tree.removeListener(this);
    file->saveIfNeeded();
}

const meta::Setting& Settings::describe(Id id)
{
    return meta::settings[static_cast<std::size_t>(id)];
}


bool Settings::isAvailable(Id id)
{
    const auto option = describe(id).requiredOption;
    if (option.empty()) return true;
    if (option == "COMPOSESIREN_SIREN_WAVES") return COMPOSESIREN_SIREN_WAVES != 0;
    if (option == "COMPOSESIREN_SETTINGS") return true;
    if (option == "COMPOSESIREN_MCP") return COMPOSESIREN_MCP != 0;
    if (option == "COMPOSESIREN_RECORD") return COMPOSESIREN_RECORD != 0;
    if (option == "COMPOSESIREN_PARK_BRIDGE") return COMPOSESIREN_PARK_BRIDGE != 0;
    if (option == "COMPOSESIREN_CLIC") return COMPOSESIREN_CLIC != 0;
    if (option == "COMPOSESIREN_DSP_CONTROL_MIDI_IN") return COMPOSESIREN_DSP_CONTROL_MIDI_IN != 0;
    if (option == "COMPOSESIREN_SIREN_CONTROL_MIDI_IN") return COMPOSESIREN_SIREN_CONTROL_MIDI_IN != 0;
    if (option == "COMPOSESIREN_RUST_DSP") return COMPOSESIREN_RUST_DSP != 0;
    return false; // an option this build doesn't know about
}

juce::Identifier Settings::key(Id id)
{
    const auto name = describe(id).id;
    return juce::Identifier(juce::String(name.data(), name.size()));
}

// Clamped to the setting's range, as the type the property components expect.
juce::var Settings::normalised(Id id, const juce::var& v)
{
    const auto& d = describe(id);
    if (d.type == meta::SettingType::String)
        return v.toString();
    const double x = juce::jlimit(d.minimum, d.maximum, static_cast<double>(v));
    switch (d.type) {
        case meta::SettingType::Bool:   return x >= 0.5;
        case meta::SettingType::Int:
        case meta::SettingType::Choice: return static_cast<int>(std::lround(x));
        case meta::SettingType::Float:  return x;
        case meta::SettingType::String: break;
    }
    return x;
}

double Settings::get(Id id) const
{
    return static_cast<double>(tree.getProperty(key(id), describe(id).defaultValue));
}

juce::String Settings::getString(Id id) const
{
    return tree.getProperty(key(id), juce::String()).toString();
}

void Settings::set(Id id, double value)
{
    tree.setProperty(key(id), normalised(id, value), nullptr);
}

void Settings::setString(Id id, const juce::String& value)
{
    tree.setProperty(key(id), normalised(id, value), nullptr);
}

void Settings::resetToDefaults()
{
    for (std::size_t i = 0; i < settingCount; ++i) {
        const auto id = idAt(i);
        if (describe(id).type == meta::SettingType::String)
            setString(id, juce::String());
        else
            set(id, describe(id).defaultValue);
    }
}

juce::Value Settings::getValueObject(Id id)
{
    return tree.getPropertyAsValue(key(id), nullptr);
}

void Settings::valueTreePropertyChanged(juce::ValueTree&, const juce::Identifier& property)
{
    if (normalising) return;
    for (std::size_t i = 0; i < settingCount; ++i) {
        const auto id = idAt(i);
        if (key(id) != property) continue;

        // values typed in a property component arrive unclamped
        const auto current = tree.getProperty(property);
        const auto clean = normalised(id, current);
        if (clean != current || clean.isBool() != current.isBool()) {
            const juce::ScopedValueSetter<bool> guard(normalising, true);
            tree.setProperty(property, clean, nullptr);
        }
        if (describe(id).type == meta::SettingType::String)
            file->setValue(property.toString(), clean.toString());
        else
            file->setValue(property.toString(), static_cast<double>(clean));
#if COMPOSESIREN_RUST_DSP
        if (id == meta::SettingId::dspVariant || id == meta::SettingId::dspDataset || id == meta::SettingId::dspSkipSilentFrames)
            publishDspChoice(*this);
#endif
        listeners.call([id](Listener& l) { l.settingChanged(id); });
        return;
    }
}

} // namespace cs

namespace cs {

bool Settings::appliesTo(Id id, std::string_view variant)
{
    std::string_view rest = describe(id).appliesTo;
    if (rest.empty()) return true;
    for (;;) {
        const auto bar = rest.find('|');
        if (rest.substr(0, bar) == variant) return true;
        if (bar == std::string_view::npos) return false;
        rest.remove_prefix(bar + 1);
    }
}

std::string Settings::dspVariant() const
{
#if COMPOSESIREN_RUST_DSP
    return choiceAt(describe(Id::dspVariant), getInt(Id::dspVariant));
#else
    return "cpp";
#endif
}

} // namespace cs
