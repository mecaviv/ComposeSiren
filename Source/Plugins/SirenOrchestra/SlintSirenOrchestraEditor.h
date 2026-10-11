//
// SirenOrchestra editor drawn by the Slint UI (Rust, Source/composesiren-slint-ui, src/orchestra.rs), built
// with -DCOMPOSESIREN_SLINT_UI=ON. Like SlintOneSirenEditor: JUCE keeps the audio, the parameters, the
// formats and the window; the pixels come from Slint's software renderer, the mouse goes to Slint.
//
// The parameter table comes from the Rust side (generated metadata): one juce::ParameterAttachment per
// entry, looked up by its JUCE id ("S5 | Volume", "R | ReverbDryWet", "C | ClicVolume", ...).
//

#ifndef SIRENORCHESTRA_SLINTSIRENORCHESTRAEDITOR_H
#define SIRENORCHESTRA_SLINTSIRENORCHESTRAEDITOR_H

#include <memory>
#include <optional>
#include <vector>

#include <juce_audio_processors/juce_audio_processors.h>
#include <composesiren_slint_ui.h>

#include "PluginProcessor.h"
#include "PluginEditor.h" // sirenOrder

class SlintSirenOrchestraEditor : public juce::AudioProcessorEditor,
                                  private juce::Timer,
                                  private VoiceManagerState::Listener
{
public:
    explicit SlintSirenOrchestraEditor(SirenOrchestraPluginProcessor& p) :
        juce::AudioProcessorEditor(&p),
        audioProcessor(p)
    {
        scale = displayScale();
        ui.reset(cs_slint_orch_new(callbacks(), COMPOSESIREN_CLIC != 0, COMPOSESIREN_PARK_BRIDGE != 0, scale));
        jassert(ui != nullptr);
        pixels = juce::Image(juce::Image::ARGB,
                             static_cast<int>(cs_slint_orch_width(ui.get())),
                             static_cast<int>(cs_slint_orch_height(ui.get())),
                             true,
                             juce::SoftwareImageType());

        auto& apvts = audioProcessor.getAudioProcessorValueTreeState();
        const auto count = cs_slint_orch_param_count(ui.get());
        attachments.resize(count);
        for (uint32_t i = 0; i < count; ++i) {
            if (auto* parameter = apvts.getParameter(cs_slint_orch_param_id(ui.get(), i))) {
                attachments[i] = std::make_unique<juce::ParameterAttachment>(
                    *parameter,
                    [this, i](float value) { cs_slint_orch_set_param(ui.get(), i, value); });
                attachments[i]->sendInitialUpdate();
            }
        }

        auto& vms = audioProcessor.getVoiceManagerState();
        vms.addListener(VoiceManagerState::Listener::Key::midiInput, this);
        midiInputChanged(vms.getMidiInput());

        setSize(static_cast<int>(static_cast<float>(pixels.getWidth()) / scale),
                static_cast<int>(static_cast<float>(pixels.getHeight()) / scale));
        startTimerHz(60);
    }

    ~SlintSirenOrchestraEditor() override
    {
        stopTimer();
        audioProcessor.getVoiceManagerState().removeListener(VoiceManagerState::Listener::Key::midiInput, this);
        attachments.clear();
        ui.reset();
    }

    void paint(juce::Graphics& g) override { g.drawImage(pixels, getLocalBounds().toFloat()); }

    void mouseDown(const juce::MouseEvent& e) override  { pointer(CS_POINTER_DOWN, e); }
    void mouseUp(const juce::MouseEvent& e) override    { pointer(CS_POINTER_UP, e); }
    void mouseDrag(const juce::MouseEvent& e) override  { pointer(CS_POINTER_MOVE, e); }
    void mouseMove(const juce::MouseEvent& e) override  { pointer(CS_POINTER_MOVE, e); }
    void mouseExit(const juce::MouseEvent& e) override  { pointer(CS_POINTER_EXIT, e); }

    void mouseWheelMove(const juce::MouseEvent& e, const juce::MouseWheelDetails& wheel) override
    {
        cs_slint_orch_wheel(ui.get(), e.position.x, e.position.y, wheel.deltaX * 100.0f, wheel.deltaY * 100.0f);
    }

private:
    struct UiDeleter {
        void operator()(CsSlintOrch* p) const { cs_slint_orch_free(p); }
    };

    static float displayScale()
    {
        if (auto* display = juce::Desktop::getInstance().getDisplays().getPrimaryDisplay()) {
            return static_cast<float>(display->scale);
        }
        return 1.0f;
    }

    static SlintSirenOrchestraEditor& self(void* context) { return *static_cast<SlintSirenOrchestraEditor*>(context); }

    CsSlintOrchCallbacks callbacks()
    {
        CsSlintOrchCallbacks c {};
        c.context = this;
        c.param_changed = [](void* context, uint32_t param, float value) {
            auto& a = self(context).attachments;
            if (param < a.size() && a[param]) { a[param]->setValueAsPartOfGesture(value); }
        };
        c.gesture = [](void* context, uint32_t param, bool begin) {
            auto& a = self(context).attachments;
            if (param < a.size() && a[param]) { begin ? a[param]->beginGesture() : a[param]->endGesture(); }
        };
        c.track_selected = [](void* context, uint32_t track) {
            // as SirenOrchestraPluginEditor::sirenStripMenuItemSelected: the siren to reset, the MIDI input
            auto& e = self(context);
            if (track >= sirenOrder.size()) { return; }
            e.selected = sirenOrder[track];
            AnyOrOneBasedMidiChannel ch { false, sirenPropertiesById.at(sirenOrder[track])->oneBasedMidiChannel };
            e.audioProcessor.getVoiceManagerState().setMidiInput(ch, true);
        };
        c.reset = [](void* context, bool all) {
            auto& e = self(context);
            e.audioProcessor.resetSiren(all ? std::nullopt : e.selected);
        };
        c.menu = [](void*) {
            // TODO: MainButtonsComponent's Menu (Settings..., Record..., resources directory, About...)
        };
        c.park_switch = [](void* context, bool st, bool on) {
#if COMPOSESIREN_PARK_BRIDGE
            auto& p = self(context).audioProcessor;
            st ? p.stAllSwitched(on) : p.physicalSirensSwitched(on);
#else
            juce::ignoreUnused(context, st, on);
#endif
        };
        c.note = [](void* context, uint8_t note, bool on) {
            auto& e = self(context);
            const auto inch = e.audioProcessor.getVoiceManagerState().getMidiInput();
            const int channel = inch.isAny ? 1 : inch.channel.oneBased;
            auto& keyboard = e.audioProcessor.getMidiKeyboardState();
            on ? keyboard.noteOn(channel, note, 0.8f) : keyboard.noteOff(channel, note, 0.0f);
        };
        return c;
    }

    void timerCallback() override
    {
        CsRect dirty {};
        bool redrawn = false;
        {
            juce::Image::BitmapData data(pixels, juce::Image::BitmapData::readWrite);
            redrawn = cs_slint_orch_tick_region(ui.get(), data.data,
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
        cs_slint_orch_pointer(ui.get(), kind, e.position.x, e.position.y);
    }

    // VoiceManagerState::Listener: the selected track follows the MIDI input channel ("Any" is channel 1)
    void midiInputChanged(AnyOrOneBasedMidiChannel inch) override
    {
        const auto sid = sirenPropertiesByChannel.at(inch.channel)->id;
        selected = sid;
        for (uint32_t t = 0; t < sirenOrder.size(); ++t) {
            if (sirenOrder[t] == sid) { cs_slint_orch_select_track(ui.get(), t); }
        }
    }

    SirenOrchestraPluginProcessor& audioProcessor;
    std::unique_ptr<CsSlintOrch, UiDeleter> ui;
    juce::Image pixels;
    float scale = 1.0f;
    std::vector<std::unique_ptr<juce::ParameterAttachment>> attachments;
    std::optional<sirenId> selected;
};

#endif //SIRENORCHESTRA_SLINTSIRENORCHESTRAEDITOR_H
