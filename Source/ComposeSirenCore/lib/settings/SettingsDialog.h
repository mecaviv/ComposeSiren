#pragma once

// The Settings... window (COMPOSESIREN_SETTINGS). Built from the settings
// metadata: one section per group, one row per setting this build has, with
// the description as tooltip. A stand-in until the settings GUI is decided.

#if COMPOSESIREN_SETTINGS

#include <juce_gui_basics/juce_gui_basics.h>
#include "Settings.h"

namespace cs {

class SettingsDialog : public juce::Component
{
public:
    SettingsDialog();

    // Opens the window (not modal), near `parent`.
    static void show(juce::Component* parent);

    void resized() override;

private:
    juce::SharedResourcePointer<Settings> settings;
    juce::PropertyPanel panel;
    juce::TextButton reset { "Reset to defaults" };
    juce::TooltipWindow tooltips { this };
};

} // namespace cs

#endif
