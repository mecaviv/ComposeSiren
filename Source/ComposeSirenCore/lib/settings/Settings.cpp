#include "Settings.h"

#ifndef COMPOSESIREN_SIREN_WAVES
#define COMPOSESIREN_SIREN_WAVES 0
#endif

namespace cs {

namespace meta = mecaviv::metadata;

Settings::Settings()
{
#if COMPOSESIREN_SETTINGS
    juce::PropertiesFile::Options options;
    options.applicationName = "ComposeSiren";
    options.filenameSuffix = ".settings";
    options.folderName = "Mecanique Vivante";
    options.osxLibrarySubFolder = "Application Support";
    options.storageFormat = juce::PropertiesFile::storeAsXML;
    options.millisecondsBeforeSaving = 500;
    file = std::make_unique<juce::PropertiesFile>(options);
#endif

    for (std::size_t i = 0; i < settingCount; ++i) {
        const auto id = idAt(i);
        juce::var v = describe(id).type == meta::SettingType::String
                          ? juce::var(juce::String())
                          : juce::var(describe(id).defaultValue);
#if COMPOSESIREN_SETTINGS
        if (file->containsKey(key(id).toString())) {
            if (describe(id).type == meta::SettingType::String)
                v = file->getValue(key(id).toString());
            else
                v = file->getValue(key(id).toString()).getDoubleValue();
        }
#endif
        tree.setProperty(key(id), normalised(id, v), nullptr);
    }
    tree.addListener(this);
}

Settings::~Settings()
{
    tree.removeListener(this);
#if COMPOSESIREN_SETTINGS
    file->saveIfNeeded();
#endif
}

const meta::Setting& Settings::describe(Id id)
{
#if COMPOSESIREN_RESETALLCONTROLLERS && COMPOSESIREN_SETTINGS
    if (id == controllerResetEnabled) {
        static constexpr meta::Setting reset {
            "reset_all_controllers.enabled", "Controller reset", "Enable",
            meta::SettingType::Bool, 1.0, 0.0, 1.0, "", "",
            "COMPOSESIREN_RESETALLCONTROLLERS", "global",
            "Enable the experimental controller reset and MIDI CC 120/123 handling. "
            "When off, reset_controllers returns an error and MIDI handling follows the main branch."
        };
        return reset;
    }
#endif
    return meta::settings[static_cast<std::size_t>(id)];
}

bool Settings::isAvailable(Id id)
{
    const auto option = describe(id).requiredOption;
    if (option.empty()) return true;
    if (option == "COMPOSESIREN_SIREN_WAVES") return COMPOSESIREN_SIREN_WAVES != 0;
    if (option == "COMPOSESIREN_SETTINGS") return COMPOSESIREN_SETTINGS != 0;
    if (option == "COMPOSESIREN_MCP") return COMPOSESIREN_MCP != 0;
    if (option == "COMPOSESIREN_RECORD") return COMPOSESIREN_RECORD != 0;
    if (option == "COMPOSESIREN_PARK_BRIDGE") return COMPOSESIREN_PARK_BRIDGE != 0;
    if (option == "COMPOSESIREN_CLIC") return COMPOSESIREN_CLIC != 0;
#if COMPOSESIREN_RESETALLCONTROLLERS && COMPOSESIREN_SETTINGS
    if (option == "COMPOSESIREN_RESETALLCONTROLLERS") return true;
#endif
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
#if COMPOSESIREN_SETTINGS
        if (describe(id).type == meta::SettingType::String)
            file->setValue(property.toString(), clean.toString());
        else
            file->setValue(property.toString(), static_cast<double>(clean));
#endif
        listeners.call([id](Listener& l) { l.settingChanged(id); });
        return;
    }
}

} // namespace cs
