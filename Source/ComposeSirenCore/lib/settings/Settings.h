#pragma once

// Power-user settings of the plugins. What exists, with its type, range,
// default and description, comes from lib/definitions/generated/
// SettingsMetadata.h, committed, generated from mecaviv-dev-root's
// resources/metadata/composesiren-settings.csv by `franz doctor fix meta`.
//
// With COMPOSESIREN_SETTINGS the values are stored per user (a properties
// file next to the other Mecanique Vivante settings) and the Settings...
// button edits them; without it every setting keeps its default. Message
// thread only. Share one instance per process with
// juce::SharedResourcePointer<cs::Settings>.

#include <juce_data_structures/juce_data_structures.h>
#include "../definitions/generated/SettingsMetadata.h"

namespace cs {

class Settings : private juce::ValueTree::Listener
{
public:
    using Id = mecaviv::metadata::SettingId;

    class Listener
    {
    public:
        virtual ~Listener() = default;
        virtual void settingChanged(Id) = 0;
    };

    Settings();
    ~Settings() override;

    static const mecaviv::metadata::Setting& describe(Id id);
    static Id idAt(std::size_t index) { return static_cast<Id>(index); }
    // false when the CMake option the setting needs is not compiled in
    static bool isAvailable(Id id);

    double get(Id id) const;
    bool getBool(Id id) const { return get(id) >= 0.5; }
    int getInt(Id id) const { return static_cast<int>(std::lround(get(id))); }
    float getFloat(Id id) const { return static_cast<float>(get(id)); }
    /// String settings (SettingType::String); empty for the others.
    juce::String getString(Id id) const;

    void set(Id id, double value);
    void setString(Id id, const juce::String& value);
    void resetToDefaults();

    // the stored value, for juce property components
    juce::Value getValueObject(Id id);

    void addListener(Listener* l) { listeners.add(l); }
    void removeListener(Listener* l) { listeners.remove(l); }

private:
    static juce::Identifier key(Id id);
    static juce::var normalised(Id id, const juce::var& v);
    void valueTreePropertyChanged(juce::ValueTree&, const juce::Identifier&) override;

    juce::ValueTree tree { "ComposeSirenSettings" };
    juce::ListenerList<Listener> listeners;
    bool normalising = false;
#if COMPOSESIREN_SETTINGS
    std::unique_ptr<juce::PropertiesFile> file;
#endif
};

} // namespace cs
