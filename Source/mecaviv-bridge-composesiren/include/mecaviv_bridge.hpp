// mecaviv-bridge-composesiren: header-only C++ wrapper over mecaviv_bridge.h.
//
// Its member names match ComposeSiren's SirenUdpBridge, so a plugin can switch
// to it with `using SirenUdpBridge = mecaviv::Bridge;` and keep its call sites.

#pragma once

#include <cstdint>

#include "mecaviv_bridge.h"

namespace mecaviv {

class Bridge
{
public:
    static constexpr int kNumSirens = MECAVIV_BRIDGE_NUM_SIRENS;

    enum class StState : int
    {
        unknown = MECAVIV_ST_STATE_UNKNOWN,
        off = MECAVIV_ST_STATE_OFF,
        on = MECAVIV_ST_STATE_ON,
    };

    // Disabled until setEnabled(true). If the bridge cannot be created, every
    // call does nothing and getStState() returns unknown.
    Bridge() noexcept : handle(mecaviv_bridge_new()) {}
    ~Bridge() { mecaviv_bridge_free(handle); }

    Bridge(const Bridge&) = delete;
    Bridge& operator=(const Bridge&) = delete;

    bool isValid() const noexcept { return handle != nullptr; }

    void setEnabled(bool on) noexcept { mecaviv_bridge_set_enabled(handle, on); }
    bool isEnabled() const noexcept { return mecaviv_bridge_is_enabled(handle); }

    // Audio thread: lock-free, no allocation, no socket.
    bool pushMidi(std::uint8_t status, std::uint8_t d1, std::uint8_t d2) noexcept
    {
        return mecaviv_bridge_push_midi(handle, status, d1, d2);
    }

    // Siren 1..kNumSirens.
    void pushReset(int siren) noexcept
    {
        if (siren >= 1 && siren <= kNumSirens)
            mecaviv_bridge_reset(handle, static_cast<std::uint8_t>(siren));
    }

    void pushResetAll() noexcept { mecaviv_bridge_reset_all(handle); }

    void setStAll(bool on) noexcept { mecaviv_bridge_set_st_all(handle, on); }

    // Siren 1..kNumSirens.
    StState getStState(int siren) const noexcept
    {
        if (siren < 1 || siren > kNumSirens)
            return StState::unknown;
        return static_cast<StState>(
            mecaviv_bridge_st_state(handle, static_cast<std::uint8_t>(siren)));
    }

    static const char* version() noexcept { return mecaviv_bridge_version(); }

private:
    mecaviv_bridge_t* handle;
};

} // namespace mecaviv
