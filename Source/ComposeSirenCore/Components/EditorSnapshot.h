//
// Development aid (Debug builds only, COMPOSESIREN_DEV_BUILD): pick the editor and write a PNG of it,
// so the JUCE and Slint editors can be compared without screen-recording permissions.
//
//   COMPOSESIREN_EDITOR=juce      the JUCE editor even when COMPOSESIREN_SLINT_UI compiled the Slint one in
//   COMPOSESIREN_SNAPSHOT=<png>   about 2 s after the editor opens, write it to <png> (component snapshot,
//                                 at COMPOSESIREN_SNAPSHOT_SCALE, default: the main display's scale)
//   COMPOSESIREN_SNAPSHOT_QUIT=1  then quit the Standalone app
//

#pragma once

#include <juce_gui_basics/juce_gui_basics.h>

#ifndef COMPOSESIREN_DEV_BUILD
#define COMPOSESIREN_DEV_BUILD 0
#endif

namespace cs::dev {

inline juce::String env(const char* name)
{
    return juce::SystemStats::getEnvironmentVariable(name, {});
}

// True when COMPOSESIREN_EDITOR=juce asks for the JUCE editor (Debug builds only).
inline bool wantsJuceEditor()
{
#if COMPOSESIREN_DEV_BUILD
    return env("COMPOSESIREN_EDITOR").equalsIgnoreCase("juce");
#else
    return false;
#endif
}

#if COMPOSESIREN_DEV_BUILD
class EditorSnapshot : private juce::Timer
{
public:
    EditorSnapshot(juce::Component& c, juce::File f, float s, bool q)
        : editor(&c), file(std::move(f)), scale(s), quit(q)
    {
        startTimer(2000);
    }

private:
    void timerCallback() override
    {
        stopTimer();
        if (auto* c = editor.getComponent()) {
            auto image = c->createComponentSnapshot(c->getLocalBounds(), true, scale);
            file.deleteFile();
            juce::FileOutputStream out(file);
            juce::PNGImageFormat().writeImageToStream(image, out);
            DBG("snapshot: " << file.getFullPathName());
        }
        if (quit) {
            if (auto* app = juce::JUCEApplicationBase::getInstance()) {
                app->systemRequestedQuit();
            }
        }
        delete this;
    }

    juce::Component::SafePointer<juce::Component> editor;
    juce::File file;
    float scale;
    bool quit;
};
#endif

// Hand back `editor`, scheduling a snapshot of it when COMPOSESIREN_SNAPSHOT is set (Debug builds only).
template <typename Editor>
Editor* withSnapshot(Editor* editor)
{
#if COMPOSESIREN_DEV_BUILD
    const auto path = env("COMPOSESIREN_SNAPSHOT");
    if (path.isNotEmpty()) {
        const auto s = env("COMPOSESIREN_SNAPSHOT_SCALE").getFloatValue();
        float scale = 1.0f;
        if (auto* d = juce::Desktop::getInstance().getDisplays().getPrimaryDisplay()) {
            scale = static_cast<float>(d->scale);
        }
        new EditorSnapshot(*editor, juce::File(path), s > 0.0f ? s : scale,
                           env("COMPOSESIREN_SNAPSHOT_QUIT") == "1");
    }
#endif
    return editor;
}

} // namespace cs::dev
