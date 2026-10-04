#include "lib/dsp/MidiIn.h"
#include "lib/settings/Settings.h"

#include <iostream>
#include <stdexcept>

namespace {

void check(bool condition, const char* message)
{
    if (!condition) throw std::runtime_error(message);
}

void checkMidi(bool enabled)
{
    float volume = -1;
    MidiIn midi(allSirenIds.front(), [&](float v) { volume = v; }, [](float) {});
    midi.setControllerResetEnabled(enabled);
    auto play = [&] {
        midi.handleControlChange(7, 127);
        midi.realTimeStartNote(69, 100);
        check(volume > 0, "note must sound before reset");
    };

    play();
    midi.handleControlChange(120, 0);
    check(volume > 0, "characterize the current CC 120 value-zero discrepancy");
    midi.handleControlChange(120, 1);
    check(enabled ? volume == 0 : volume > 0, "CC 120 must obey the runtime switch");

    play();
    midi.handleControlChange(123, 0);
    check(volume > 0, "characterize the current CC 123 value-zero discrepancy");
    midi.handleControlChange(123, 1);
    check(enabled ? volume == 0 : volume > 0, "CC 123 must obey the runtime switch");

    for (int value : {0, 127}) {
        play();
        const auto before = volume;
        midi.handleControlChange(121, value);
        check(enabled ? volume == 0 : volume == before,
              "CC 121 keeps master behaviour when disabled, for any value");
        for (int i = 0; i < 200; ++i) midi.timerAudio();
        check(enabled ? volume == 0 : volume == before, "reset must not restart the volume ramp");
    }

    // CC 123 follows the release envelope rather than cutting immediately.
    play();
    midi.handleControlChange(72, 20);
    midi.handleControlChange(123, 1);
    check(volume > 0, "release does not cut immediately");
    for (int i = 0; i < 1000; ++i) midi.timerAudio();
    check(enabled ? volume == 0 : volume > 0, "release completes only when enabled");
}

void checkSettings()
{
#if COMPOSESIREN_SETTINGS
    const auto& row = cs::Settings::describe(cs::Settings::controllerResetEnabled);
    check(row.id == "reset_all_controllers.enabled", "stable setting storage key");
    check(row.group == "Controller reset" && row.type == mecaviv::metadata::SettingType::Bool,
          "one boolean in the reset section");
    check(row.defaultValue == 1.0 && row.scope == "global", "enabled by default, shared per process");
    check(cs::Settings::isAvailable(cs::Settings::controllerResetEnabled), "setting is available");
    check(cs::Settings::settingCount == mecaviv::metadata::settingCount + 1, "one optional row");
#else
    check(cs::Settings::settingCount == mecaviv::metadata::settingCount, "no row without settings");
#endif
}

} // namespace

int main()
{
    try {
        checkMidi(true);
        checkMidi(false);
        checkSettings();
        std::cout << "Controller reset runtime and MIDI characterization checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
