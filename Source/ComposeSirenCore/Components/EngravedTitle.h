//
// Engraved group titles for the bottom strips (Clic, Reverb).
//

#pragma once

#include <juce_graphics/juce_graphics.h>
#include <juce_gui_basics/juce_gui_basics.h>

/// Text with a painted engraving underline — a short rule that fades at both
/// ends, like a stamped panel legend. Fits strip headers.
class EngravedTitle : public juce::Component
{
public:
    void setText(const juce::String& t)
    {
        text = t;
        repaint();
    }

    void paint(juce::Graphics& g) override
    {
        auto area = getLocalBounds().toFloat();
        const auto font = juce::FontOptions(area.getHeight() * 0.55f, juce::Font::bold);
        g.setFont(font);

        const float x = 2.0f;
        const float y = area.getHeight() * 0.12f;
        const float tw = juce::jmax(0.0f, area.getWidth() - x - 2.0f);
        const float th = area.getHeight() * 0.72f;

        // light pass then dark pass = engraved look, left-aligned, full remaining width
        g.setColour(juce::Colours::white.withAlpha(0.35f));
        g.drawText(text, x, y + 0.5f, tw, th, juce::Justification::centredLeft);
        g.setColour(juce::Colours::white.withAlpha(0.92f));
        g.drawText(text, x, y, tw, th, juce::Justification::centredLeft);

        // full-width underline across the block
        const float ruleY = area.getBottom() - 1.5f;
        g.setColour(juce::Colours::white.withAlpha(0.40f));
        g.fillRect(0.0f, ruleY, area.getWidth(), 1.2f);
    }

private:
    juce::String text;
};
