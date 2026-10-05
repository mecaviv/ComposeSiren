#pragma once

#if COMPOSESIREN_RESETALLCONTROLLERS

#include <array>
#include <cstdint>
#include <optional>

namespace cs {

enum class MidiValueRule { ignore, zero, positive, any };
enum class MidiOutputValue { preserve, zero, positive };

constexpr int channelModeIndex(int cc)
{
    return cc == 120 ? 0 : cc == 121 ? 1 : cc == 123 ? 2 : -1;
}

constexpr bool acceptsMidiValue(MidiValueRule rule, int value)
{
    if (value < 0 || value > 127) return false;
    switch (rule) {
        case MidiValueRule::ignore:   return false;
        case MidiValueRule::zero:     return value == 0;
        case MidiValueRule::positive: return value > 0;
        case MidiValueRule::any:      return true;
    }
    return false;
}

// One word is published from the message thread and sampled per audio block.
struct DspChannelModePolicy {
    bool enabled = true;
    std::array<MidiValueRule, 3> values {
        MidiValueRule::positive, MidiValueRule::any, MidiValueRule::positive
    };

    constexpr bool accepts(int cc, int value) const
    {
        const auto i = channelModeIndex(cc);
        if (i < 0) return false;
        // Disabled experimentation retains the existing CC 121 behavior.
        return enabled ? acceptsMidiValue(values[static_cast<std::size_t>(i)], value) : cc == 121;
    }

    constexpr std::uint32_t packed() const
    {
        return (enabled ? 1u : 0u) | (static_cast<unsigned>(values[0]) << 1)
            | (static_cast<unsigned>(values[1]) << 3) | (static_cast<unsigned>(values[2]) << 5);
    }

    static constexpr DspChannelModePolicy unpack(std::uint32_t bits)
    {
        return { (bits & 1u) != 0, { static_cast<MidiValueRule>((bits >> 1) & 3u),
            static_cast<MidiValueRule>((bits >> 3) & 3u), static_cast<MidiValueRule>((bits >> 5) & 3u) } };
    }
};

struct BridgeChannelModePolicy {
    bool enabled = true;
    std::array<MidiValueRule, 3> values {
        MidiValueRule::any, MidiValueRule::any, MidiValueRule::any
    };
    std::array<MidiOutputValue, 3> output {
        MidiOutputValue::preserve, MidiOutputValue::preserve, MidiOutputValue::preserve
    };

    // Only the optional bridge copy changes; host MIDI and DSP input stay intact.
    constexpr std::optional<int> forwardedValue(int cc, int value) const
    {
        const auto i = channelModeIndex(cc);
        if (!enabled || i < 0) return value;
        const auto index = static_cast<std::size_t>(i);
        if (!acceptsMidiValue(values[index], value)) return std::nullopt;
        switch (output[index]) {
            case MidiOutputValue::preserve: return value;
            case MidiOutputValue::zero:     return 0;
            case MidiOutputValue::positive: return 127;
        }
        return value;
    }

    constexpr std::uint32_t packed() const
    {
        return DspChannelModePolicy { enabled, values }.packed()
            | (static_cast<unsigned>(output[0]) << 7)
            | (static_cast<unsigned>(output[1]) << 9)
            | (static_cast<unsigned>(output[2]) << 11);
    }

    static constexpr BridgeChannelModePolicy unpack(std::uint32_t bits)
    {
        const auto dsp = DspChannelModePolicy::unpack(bits);
        return { dsp.enabled, dsp.values, { static_cast<MidiOutputValue>((bits >> 7) & 3u),
            static_cast<MidiOutputValue>((bits >> 9) & 3u), static_cast<MidiOutputValue>((bits >> 11) & 3u) } };
    }
};

} // namespace cs

#endif
