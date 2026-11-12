#pragma once

// Power-user settings of the plugins. What exists, with its type, range,
// default and description, comes from lib/definitions/generated/
// SettingsMetadata.h (generated from the shared settings metadata).
//
// The values are stored per user (a properties file next to the other
// Mecanique Vivante settings) and the Settings... menu item edits them, in
// every build; a setting whose option this build lacks keeps its default and
// isn't shown (isAvailable). Message
// thread only. Share one instance per process with
// juce::SharedResourcePointer<cs::Settings>.

#include <juce_data_structures/juce_data_structures.h>
#include "../definitions/generated/SettingsMetadata.h"


namespace cs {

class Settings : private juce::ValueTree::Listener
{
public:
    using Id = mecaviv::metadata::SettingId;



    static constexpr std::size_t settingCount
        = mecaviv::metadata::settingCount;

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
    // whether the setting means something with the `dsp.variant` named `variant` (its
    // metadata's applies_to lists it, or lists none)
    static bool appliesTo(Id id, std::string_view variant);
    // the dsp.variant id selected ("cpp" without COMPOSESIREN_RUST_DSP)
    std::string dspVariant() const;

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
    std::unique_ptr<juce::PropertiesFile> file;
};

} // namespace cs
