// clic-composesiren: header-only C++ wrapper over clic_composesiren.h.

#pragma once

#include <cstddef>
#include <cstdint>

#include "clic_composesiren.h"

namespace clic {

// One plugin instance's click engine. midi() and render() neither allocate
// nor block (audio thread); setSampleRate() reloads the clicks (prepareToPlay).
class Engine
{
public:
    explicit Engine(double sampleRate) : handle(clic_new(sampleRate)) {}
    ~Engine() { clic_free(handle); }
    Engine(const Engine&) = delete;
    Engine& operator=(const Engine&) = delete;

    void setSampleRate(double sampleRate) { clic_set_sample_rate(handle, sampleRate); }

    void midi(std::uint8_t status, std::uint8_t data1, std::uint8_t data2)
    {
        clic_midi(handle, status, data1, data2);
    }

    void render(float* left, float* right, int frames)
    {
        if (frames > 0)
            clic_render(handle, left, right, static_cast<std::size_t>(frames));
    }

    /// spread +1: clic1 left, clic2 right (−1 the opposite); bias moves both.
    /// decay 1: full sample, 0: ~2 ms fade (just audible).
    void renderPanned(float* left, float* right, int frames, float spread, float bias, float decay)
    {
        if (frames > 0)
            clic_render_panned(handle, left, right, static_cast<std::size_t>(frames), spread, bias, decay);
    }

    int current() const { return clic_current(handle); }
    int count() const { return clic_count(handle); }

private:
    clic_t* handle;
};

} // namespace clic
