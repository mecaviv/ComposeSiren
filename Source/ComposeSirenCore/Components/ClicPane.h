//
// Clic control pane: header (LED)(white btn) Clic + full-width rule,
// Spread / Decay / Bias (white) and Vol (black hero).
// Header switch is a master override — it does not write ClicVolume.
// OFF ⇒ effective gain 0 regardless of the Vol knob.
//

#pragma once

#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_gui_basics/juce_gui_basics.h>

#include "EngravedTitle.h"
#include "LookAndFeels.h"
#include "SliderCell.h"
#include "ToggleCell.h"
#include "../lib/definitions/parameterDefinitions.h"
#include "../lib/definitions/palette.h"

#ifndef COMPOSESIREN_CLIC
#define COMPOSESIREN_CLIC 0
#endif

#if COMPOSESIREN_CLIC

class ClicPane : public juce::Component
{
public:
    explicit ClicPane(juce::AudioProcessorValueTreeState& vts)
    {
        // Header: () [ ] Clic — ToggleLAF button with LED on the left
        enable.setButtonText({});
        enable.setToggleable(true);
        enable.setClickingTogglesState(true);
        enable.setLookAndFeel(&headerSwitchLAF);
        addAndMakeVisible(enable);

        title.setText("Clic");
        addAndMakeVisible(title);

        // Override only — writes ClicEnable, never touches ClicVolume
        enableAttachment = std::make_unique<juce::AudioProcessorValueTreeState::ButtonAttachment>(
            vts, ParameterIdGet::toJuceParameterId("C", ParameterId::ClicEnable), enable);

        decay.setNameText("Decay");
        decay.setSliderAttachment(vts, ParameterId::ClicDecay, "C");
        decay.setShowTextBox(false);
        decay.setShowLabel(true);
        decay.getSlider().setLookAndFeel(&whiteKnob);
        decay.setSliderFillWhite();
        addAndMakeVisible(decay);

        spread.setNameText("Spread");
        spread.setSliderAttachment(vts, ParameterId::ClicSpread, "C");
        spread.setShowTextBox(false);
        spread.setShowLabel(true);
        spread.getSlider().setLookAndFeel(&whiteKnob);
        spread.setSliderFillWhite();
        addAndMakeVisible(spread);

        bias.setNameText("Bias");
        bias.setSliderAttachment(vts, ParameterId::ClicBias, "C");
        bias.setShowTextBox(false);
        bias.setShowLabel(true);
        bias.getSlider().setLookAndFeel(&whiteKnob);
        bias.setSliderFillWhite();
        addAndMakeVisible(bias);

        volume.setNameText("Vol");
        volume.setSliderAttachment(vts, ParameterId::ClicVolume, "C");
        volume.getSlider().setLookAndFeel(&blackKnob);
        volume.setShowTextBox(false);
        volume.setShowLabel(true);
        addAndMakeVisible(volume);
    }

    ~ClicPane() override
    {
        enable.setLookAndFeel(nullptr);
        decay.getSlider().setLookAndFeel(nullptr);
        spread.getSlider().setLookAndFeel(nullptr);
        bias.getSlider().setLookAndFeel(nullptr);
        volume.getSlider().setLookAndFeel(nullptr);
    }

    void setBackgroundStripColour(juce::Colour c)
    {
        backgroundStripColour = c;
        repaint();
    }

    void resized() override
    {
        auto area = getLocalBounds().reduced(4);

        // header: () [ ] Clic
        auto header = area.removeFromTop(22);
        enable.setBounds(header.removeFromLeft(40));
        title.setBounds(header);

        // Spread / Decay / Bias close under the underline
        const int decayW = 48, decayH = 52;
        decay.setBounds((getWidth() - decayW) / 2, 28, decayW, decayH);

        const int sideW = 40, sideH = 44;
        spread.setBounds(2, 30, sideW, sideH);
        bias.setBounds(getWidth() - sideW - 2, 30, sideW, sideH);

        // Vol: label stays at y=82; knob is smaller and sits lower
        const int volW = 52, volH = 64;
        volume.setBounds((getWidth() - volW) / 2, 82, volW, volH);
        volume.setMinHeight(42.f);
    }

    void paint(juce::Graphics& g) override
    {
        // same chassis as Master Volume (bottom siren colour, not glass)
        g.setColour(backgroundStripColour);
        g.fillRoundedRectangle(getLocalBounds().toFloat(), controlStripLayout::cornerSize);
    }

private:
    EngravedTitle title;
    juce::ToggleButton enable;
    std::unique_ptr<juce::AudioProcessorValueTreeState::ButtonAttachment> enableAttachment;

    juce::Colour backgroundStripColour { juce::Colour { mecaviv::Colours::backgroundStripGrey } };

    SliderCell decay  { 44.f, 52.f, 14.f };
    SliderCell spread { 30.f, 42.f, 12.f };
    SliderCell bias   { 30.f, 42.f, 12.f };
    SliderCell volume { 52.f, 42.f, 14.f };

    HeaderSwitchLAF headerSwitchLAF;
    KnobLAF whiteKnob;
    NotchedKnobLAF blackKnob;
};

#endif // COMPOSESIREN_CLIC
