#include "lib/dsp/MidiIn.h"
#include <iostream>
#include <stdexcept>

int main()
{
    try {
        for (int value = 0; value <= 127; ++value) {
            float volume = -1;
            float note = -1;
            MidiIn midi(allSirenIds.front(), [&](float v) { volume = v; }, [&](float n) { note = n; });
            midi.realTimeStartNote(69, 100);
            midi.handleControlChange(7, 40);
            midi.handlePitchWheel(0, 96);
            midi.handleControlChange(121, value);
            if (volume <= 0) throw std::runtime_error("CC 121 released the held note");
            midi.realTimeStopNote(69);
            if (volume != 0) throw std::runtime_error("CC 121 lost the note identity");
            midi.realTimeStartNote(69, 100);
            midi.handleControlChange(7, 40);
            const auto before = volume;
            midi.handleControlChange(123, value);
            if (volume != 0) throw std::runtime_error("CC 123 did not release");
            midi.realTimeStartNote(69, 100);
            if (volume != before) throw std::runtime_error("CC 123 changed controllers");
            midi.handleControlChange(120, value);
            if (volume != 0) throw std::runtime_error("CC 120 did not silence");
            if (note != 0) throw std::runtime_error("CC 120 did not stop the motor");
            midi.realTimeStartNote(69, 100);
            if (volume != before) throw std::runtime_error("CC 120 changed controllers");
        }
        std::cout << "Channel-mode checks passed for every value\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
