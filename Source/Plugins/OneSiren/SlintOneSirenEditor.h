//
// OneSiren editor drawn by the Slint UI (Rust, Source/composesiren-slint-ui), built with
// -DCOMPOSESIREN_SLINT_UI=ON. JUCE keeps the audio, the parameters (AudioProcessorValueTreeState), the
// plugin formats and the window: this editor is an ordinary juce::AudioProcessorEditor whose pixels come
// from Slint's software renderer and whose mouse events go to Slint. No second native window.
//
// Thread: everything here runs on JUCE's message thread, like the Slint side (see src/embed.rs).
//

#ifndef ONESIREN_SLINTONESIRENEDITOR_H
#define ONESIREN_SLINTONESIRENEDITOR_H

#include <array>
#include <memory>

#include <juce_audio_processors/juce_audio_processors.h>
#include <composesiren_slint_ui.h>

#include "PluginProcessor.h"

class SlintOneSirenEditor : public juce::AudioProcessorEditor,
                            private juce::Timer,
                            private VoiceManagerState::Listener
{
public:
    explicit SlintOneSirenEditor(OneSirenPluginProcessor& p) :
        juce::AudioProcessorEditor(&p),
        audioProcessor(p)
    {
        scale = displayScale();
        ui.reset(cs_slint_ui_new(callbacks(), scale));
        jassert(ui != nullptr);
        pixels = juce::Image(juce::Image::ARGB,
                             static_cast<int>(cs_slint_ui_width(ui.get())),
                             static_cast<int>(cs_slint_ui_height(ui.get())),
                             true,
                             juce::SoftwareImageType()); // CPU pixels: no GPU readback (Direct2D) per frame

        // One attachment per parameter of the strip: host/automation/MIDI-in values reach the UI through
        // the attachment callback (message thread), UI edits go out through the same attachment.
        auto& apvts = audioProcessor.getAudioProcessorValueTreeState();
        const std::string groupId = audioProcessor.getParameterLayoutData()[0].id;
        for (uint32_t i = 0; i < cs_slint_ui_param_count(); ++i) {
            const std::string id = groupId + parameterGroupSeparator + cs_slint_ui_param_code_name(i);
            if (auto* parameter = apvts.getParameter(id)) {
                attachments[i] = std::make_unique<juce::ParameterAttachment>(
                    *parameter,
                    [this, i](float value) { cs_slint_ui_set_param(ui.get(), i, value); });
                attachments[i]->sendInitialUpdate();
            }
        }

        auto& voiceManager = audioProcessor.getVoiceManagerState();
        voiceManager.addListener(VoiceManagerState::Listener::Key::category, this);
        categoryChanged(voiceManager.getSirenCategory());

        setSize(static_cast<int>(static_cast<float>(pixels.getWidth()) / scale),
                static_cast<int>(static_cast<float>(pixels.getHeight()) / scale)); // 754 x 200, the JUCE editor's
        startTimerHz(60);
    }

    ~SlintOneSirenEditor() override
    {
        stopTimer();
        audioProcessor.getVoiceManagerState().removeListener(VoiceManagerState::Listener::Key::category, this);
        attachments = {};
        ui.reset();
    }

    void paint(juce::Graphics& g) override
    {
        g.drawImage(pixels, getLocalBounds().toFloat());
    }

    void mouseDown(const juce::MouseEvent& e) override  { pointer(CS_POINTER_DOWN, e); }
    void mouseUp(const juce::MouseEvent& e) override    { pointer(CS_POINTER_UP, e); }
    void mouseDrag(const juce::MouseEvent& e) override  { pointer(CS_POINTER_MOVE, e); }
    void mouseMove(const juce::MouseEvent& e) override  { pointer(CS_POINTER_MOVE, e); }
    void mouseExit(const juce::MouseEvent& e) override  { pointer(CS_POINTER_EXIT, e); }

    void mouseWheelMove(const juce::MouseEvent& e, const juce::MouseWheelDetails& wheel) override
    {
        // JUCE gives about 0.1 per notch; Slint wants pixels.
        cs_slint_ui_wheel(ui.get(), e.position.x, e.position.y, wheel.deltaX * 100.0f, wheel.deltaY * 100.0f);
    }

private:
    struct UiDeleter {
        void operator()(CsSlintUi* p) const { cs_slint_ui_free(p); }
    };

    static constexpr size_t parameterCount = 14; // cs_slint_ui_param_count()

    static float displayScale()
    {
        if (auto* display = juce::Desktop::getInstance().getDisplays().getPrimaryDisplay()) {
            return static_cast<float>(display->scale);
        }
        return 1.0f;
    }

    CsSlintUiCallbacks callbacks()
    {
        CsSlintUiCallbacks c {};
        c.context = this;
        c.param_changed = [](void* context, uint32_t param, float value) {
            if (auto& a = static_cast<SlintOneSirenEditor*>(context)->attachments[param]) {
                a->setValueAsPartOfGesture(value);
            }
        };
        c.gesture = [](void* context, uint32_t param, bool begin) {
            if (auto& a = static_cast<SlintOneSirenEditor*>(context)->attachments[param]) {
                begin ? a->beginGesture() : a->endGesture();
            }
        };
        c.category_changed = [](void* context, uint32_t category) {
            // same path as VoiceManagerComponent's menu (Alto, Bass, Tenor, Soprano, Piccolo)
            static_cast<SlintOneSirenEditor*>(context)->audioProcessor.getVoiceManagerState()
                .setSirenCategory(static_cast<sirenCategory>(category), true);
        };
        c.note = [](void* context, uint8_t note, bool on) {
            auto& keyboard = static_cast<SlintOneSirenEditor*>(context)->audioProcessor.getMidiKeyboardState();
            on ? keyboard.noteOn(1, note, 0.8f) : keyboard.noteOff(1, note, 0.0f);
        };
        return c;
    }

    void timerCallback() override
    {
        // Slint draws only when something changed (an edit, a host value, an animation), and only that
        // part of the image (the image keeps the rest): repaint that rectangle, in logical pixels.
        CsRect dirty {};
        bool redrawn = false;
        {
            juce::Image::BitmapData data(pixels, juce::Image::BitmapData::readWrite);
            redrawn = cs_slint_ui_tick_region(ui.get(), data.data,
                                              static_cast<uint32_t>(data.lineStride / data.pixelStride), &dirty);
        }
        if (redrawn) {
            repaint(juce::Rectangle<float>(static_cast<float>(dirty.x), static_cast<float>(dirty.y),
                                           static_cast<float>(dirty.width), static_cast<float>(dirty.height))
                        .transformedBy(juce::AffineTransform::scale(1.0f / scale))
                        .getSmallestIntegerContainer());
        }
    }

    void pointer(CsPointer kind, const juce::MouseEvent& e)
    {
        cs_slint_ui_pointer(ui.get(), kind, e.position.x, e.position.y);
    }

    // VoiceManagerState::Listener
    void categoryChanged(sirenCategory category) override
    {
        cs_slint_ui_set_category(ui.get(), static_cast<uint32_t>(category));
    }

    OneSirenPluginProcessor& audioProcessor;
    std::unique_ptr<CsSlintUi, UiDeleter> ui;
    juce::Image pixels;
    float scale = 1.0f;
    std::array<std::unique_ptr<juce::ParameterAttachment>, parameterCount> attachments;
};

#endif //ONESIREN_SLINTONESIRENEDITOR_H
