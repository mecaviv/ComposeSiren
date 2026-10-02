//
// Created by joseph larralde on 21/03/2026.
//

#ifndef COMPOSESIREN_RESETCOMPONENT_H
#define COMPOSESIREN_RESETCOMPONENT_H

#include <juce_gui_basics/juce_gui_basics.h>
#include "../lib/definitions/palette.h"
#include "../lib/definitions/sirenProperties.h"
#include "../lib/net/mcp/McpControl.h"

#ifndef COMPOSESIREN_PARK_BRIDGE
#define COMPOSESIREN_PARK_BRIDGE 0
#endif
#include "../lib/utilities/recorder/RecordDialog.h"

class MainButtonsComponent : public juce::Component,
                             public juce::TextButton::Listener,
                             private juce::Timer,
                             private juce::ChangeListener
{
public:
    class Listener {
    public:
        virtual ~Listener() = default;
        virtual void resetSiren(std::optional<sirenId>) = 0;
        virtual void selectedNewResourcesPath(const std::string&) = 0;
        virtual std::string getResourcesPath() = 0;
        // optionnel : pilotage des sirènes physiques — no-op par défaut.
        // physicalSirensEnabled() donne l'état courant pour initialiser la case
        // quand l'éditeur s'ouvre (l'état vit dans le processor, pas dans l'UI).
        virtual void physicalSirensSwitched(bool) {}
        virtual bool physicalSirensEnabled() { return false; }
        virtual juce::String physicalSirensTooltip() { return {}; }
        virtual void stAllSwitched(bool) {}
        // optionnel : un bouton About... ouvre la fenêtre que showAbout() ouvre.
        virtual bool hasAbout() { return false; }
        virtual void showAbout(juce::Component* /*parent*/) {}
        // optionnel : le serveur MCP, pour afficher le morceau en cours
        // (set_song_title / clear_song_title) dans la barre de titre.
        virtual McpControl* getMcpControl() { return nullptr; }
#if COMPOSESIREN_RECORD
        // optionnel : l'enregistreur de la sortie audio ; un bouton Record...
        // ouvre son dialogue quand il y en a un.
        virtual Recorder* getRecorder() { return nullptr; }
#endif
    };

    MainButtonsComponent(Listener& l, bool hasResetAll = false,
                         bool hasStAll = false) :
        listener(l),
        hasResetAllButton(hasResetAll)
#if COMPOSESIREN_PARK_BRIDGE
        , hasStAllSwitch(hasStAll)
#endif
    {
#if !COMPOSESIREN_PARK_BRIDGE
        juce::ignoreUnused(hasStAll);
#endif
        selectResourcesButton.setColour(
            juce::TextButton::buttonColourId,
            juce::Colour{mecaviv::Colours::darkTransparentBackground}
        );
        selectResourcesButton.setColour(juce::TextButton::textColourOffId , juce::Colours::whitesmoke);
        selectResourcesButton.setButtonText("Set resources directory");
        selectResourcesButton.addListener(this);
#if COMPOSESIREN_DEV_BUILD
        addAndMakeVisible(selectResourcesButton);
#endif

        resetButton.setColour(juce::TextButton::buttonColourId, juce::Colours::darkred);
        resetButton.setColour(juce::TextButton::textColourOffId , juce::Colours::whitesmoke);
        resetButton.setButtonText ("Reset");
        resetButton.addListener(this);
        addAndMakeVisible(resetButton);

#if COMPOSESIREN_RECORD
        if (listener.getRecorder() != nullptr) {
            recordButton.setColour(juce::TextButton::buttonColourId, juce::Colour { 0xff37474f });
            recordButton.setColour(juce::TextButton::textColourOffId, juce::Colours::whitesmoke);
            recordButton.setButtonText("Record...");
            recordButton.addListener(this);
            addAndMakeVisible(recordButton);
        }
#endif

        if (listener.hasAbout()) {
            aboutButton.setColour(juce::TextButton::buttonColourId, juce::Colour { 0xff37474f });
            aboutButton.setColour(juce::TextButton::textColourOffId, juce::Colours::whitesmoke);
            aboutButton.setButtonText("About...");
            aboutButton.addListener(this);
            addAndMakeVisible(aboutButton);
        }

        if (hasResetAllButton) {
            resetAllButton.setColour(juce::TextButton::buttonColourId, juce::Colours::darkred);
            resetAllButton.setColour(juce::TextButton::textColourOffId , juce::Colours::whitesmoke);
            resetAllButton.setButtonText ("Reset All");
            resetAllButton.addListener(this);
            addAndMakeVisible(resetAllButton);
        }

#if COMPOSESIREN_PARK_BRIDGE
        if (hasStAllSwitch) {
            physicalButton.setButtonText("Sirenes physiques");
            physicalButton.setColour(juce::ToggleButton::textColourId, juce::Colours::whitesmoke);
            physicalButton.setColour(juce::ToggleButton::tickColourId, juce::Colours::whitesmoke);
            physicalButton.setToggleState(listener.physicalSirensEnabled(),
                                          juce::dontSendNotification);
            physicalButton.setTooltip(listener.physicalSirensTooltip());
            physicalButton.addListener(this);
            addAndMakeVisible(physicalButton);

            stAllButton.setButtonText("ST");
            stAllButton.setEnabled(physicalButton.getToggleState());
            stAllButton.setColour(juce::ToggleButton::textColourId, juce::Colours::whitesmoke);
            stAllButton.setColour(juce::ToggleButton::tickColourId, juce::Colours::whitesmoke);
            stAllButton.addListener(this);
            addAndMakeVisible(stAllButton);
        }
#endif

        // The song tap-viewer is playing (set_song_title), on the left of the
        // top row: its title with an ASCII progress bar, and the same line in
        // the window's title bar. Hidden again by clear_song_title.
        songTitle.setJustificationType(juce::Justification::centredLeft);
        songTitle.setColour(juce::Label::textColourId, juce::Colours::whitesmoke);
        songTitle.setFont(juce::FontOptions(13.0f, juce::Font::bold));
        songTitle.setInterceptsMouseClicks(false, false);
        addChildComponent(songTitle);

        if (auto* mcp = listener.getMcpControl()) {
            mcp->addChangeListener(this);
            applySongDisplay(mcp->getSongDisplay(), false);
        }
    }

    ~MainButtonsComponent() override
    {
        if (auto* mcp = listener.getMcpControl())
            mcp->removeChangeListener(this);
    }

    void paint(juce::Graphics& g) override
    {
        // g.setColour(juce::Colour{0xff314159});
        // g.fillRect(getLocalBounds().toFloat());
    }

    void resized() override
    {
        constexpr float margin = 5;
        auto bounds = getLocalBounds().reduced(static_cast<int>(margin));

        juce::FlexBox fb;

        fb.flexDirection = juce::FlexBox::Direction::row;
        fb.flexWrap = juce::FlexBox::Wrap::noWrap;
        // fb.alignItems = juce::FlexBox::AlignItems::center;
        fb.alignItems = juce::FlexBox::AlignItems::flexEnd;
        fb.alignContent = juce::FlexBox::AlignContent::spaceBetween;

        const float btnsHeight = static_cast<float>(bounds.getHeight());
        juce::FlexItem item;

        // The song title with its ASCII progress bar, when one is playing.
        if (songTitle.isVisible())
            songTitle.setBounds(bounds.removeFromLeft(
                static_cast<int>(juce::jmax(160.0f, bounds.getWidth() * 0.42f))));

        // Left button /////////////////////////////////////////////////////////
        // fb.alignContent = juce::FlexBox::AlignContent::flexStart;
        // fb.justifyContent = juce::FlexBox::JustifyContent::flexStart;
        //
        // item = juce::FlexItem(resetButton).withMinWidth(75)
        //                                   .withMinHeight(menuHeight)
        //                                   .withFlex(0,0);
        // fb.items.add(item);
        // fb.performLayout(bounds);
        // fb.items.clear();

        // Right button ////////////////////////////////////////////////////////
        // fb.alignContent = juce::FlexBox::AlignContent::flexEnd;
        // fb.justifyContent = juce::FlexBox::JustifyContent::flexEnd;
        //
        // item = juce::FlexItem(selectResourcesButton).withMinWidth(230)
        //                                             .withMinHeight(menuHeight)
        //                                             .withFlex(0,1);
        // fb.items.add(item);
        // fb.performLayout(bounds);

        // All buttons right ///////////////////////////////////////////////////
        fb.alignContent = juce::FlexBox::AlignContent::flexEnd;
        fb.justifyContent = juce::FlexBox::JustifyContent::flexEnd;

        item = juce::FlexItem(selectResourcesButton).withMinWidth(230)
                                                    .withMinHeight(btnsHeight)
                                                    .withFlex(0,1);
        fb.items.add(item);
#if COMPOSESIREN_RECORD
        if (recordButton.isVisible()) {
            item = juce::FlexItem(recordButton).withMinWidth(90)
                                               .withMinHeight(btnsHeight)
                                               .withFlex(0,0);
            item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
            fb.items.add(item);
        }
#endif
        if (aboutButton.isVisible()) {
            item = juce::FlexItem(aboutButton).withMinWidth(90)
                                              .withMinHeight(btnsHeight)
                                              .withFlex(0,0);
            item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
            fb.items.add(item);
        }
        item = juce::FlexItem(resetButton).withMinWidth(75)
                                          .withMinHeight(btnsHeight)
                                          .withFlex(0,0);
        item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
        fb.items.add(item);

        if (hasResetAllButton) {
            item = juce::FlexItem(resetAllButton).withMinWidth(150)
                                                 .withMinHeight(btnsHeight)
                                                 .withFlex(0,0);
            item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
            fb.items.add(item);
        }

#if COMPOSESIREN_PARK_BRIDGE
        if (hasStAllSwitch) {
            item = juce::FlexItem(physicalButton).withMinWidth(150)
                                                 .withMinHeight(btnsHeight)
                                                 .withFlex(0,0);
            item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
            fb.items.add(item);
            item = juce::FlexItem(stAllButton).withMinWidth(55)
                                              .withMinHeight(btnsHeight)
                                              .withFlex(0,0);
            item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
            fb.items.add(item);
        }
#endif

        fb.performLayout(bounds);
    }

    void setSirenIdToReset(std::optional<sirenId> id)
    {
        currentSirenId = id;
    }

private:
    // The song display: title + ASCII progress bar on the left of the top
    // row, and the same line in the window's title bar. `clear_song_title`
    // puts both back.
    void changeListenerCallback(juce::ChangeBroadcaster*) override
    {
        if (auto* mcp = listener.getMcpControl())
            applySongDisplay(mcp->getSongDisplay(), true);
    }

    void timerCallback() override
    {
        if (!song.active)
            return;
        updateSongProgress();
    }

    void applySongDisplay(const SongDisplay& next, bool relayout)
    {
        const bool wasActive = song.active;
        song = next;
        songTitle.setVisible(song.active);
        updateSongProgress();

        if (song.active != wasActive)
            resized();
        else if (relayout)
            resized();

        if (song.active)
            startTimerHz(15);
        else
            stopTimer();
    }

    // Compact ASCII art for the bar: Title [=======|-------] 1:23
    static juce::String formatSongLine(const SongDisplay& song)
    {
        if (!song.active || song.title.isEmpty())
            return {};

        juce::String line = song.title;
        if (song.durationSeconds > 0.0) {
            constexpr int cells = 14;
            const double fraction =
                juce::jlimit(0.0, 1.0, song.currentPosition() / song.durationSeconds);
            const int filled = juce::roundToInt(fraction * cells);

            line << "  [";
            for (int i = 0; i < cells; ++i) {
                if (i < filled)
                    line << '=';
                else if (i == filled)
                    line << '|';
                else
                    line << '-';
            }
            line << "]";

            const int total = juce::roundToInt(song.durationSeconds);
            const int at = juce::roundToInt(juce::jmin(song.currentPosition(),
                                                       song.durationSeconds));
            line << "  " << (at / 60) << ":"
                 << juce::String(at % 60).paddedLeft('0', 2) << "/"
                 << (total / 60) << ":"
                 << juce::String(total % 60).paddedLeft('0', 2);
        }
        return line;
    }

    void updateSongProgress()
    {
        const auto line = formatSongLine(song);
        songTitle.setText(line, juce::dontSendNotification);
        updateWindowTitle(line);
    }

    void updateWindowTitle(const juce::String& line)
    {
        auto* top = getTopLevelComponent();
        if (top == nullptr)
            return;
        if (defaultWindowTitle.isEmpty())
            defaultWindowTitle = top->getName();
        top->setName(song.active && line.isNotEmpty() ? line : defaultWindowTitle);
    }

public:

#if COMPOSESIREN_PARK_BRIDGE
    void refreshPhysicalSirensTooltip()
    {
        if (hasStAllSwitch)
            physicalButton.setTooltip(listener.physicalSirensTooltip());
    }
#endif

    void buttonClicked(juce::Button* btn) override
    {
#if COMPOSESIREN_RECORD
        if (btn == &recordButton) {
            if (auto* recorder = listener.getRecorder())
                RecordDialog::show(*recorder, getTopLevelComponent());
            return;
        }
#endif
        if (btn == &aboutButton) {
            listener.showAbout(getTopLevelComponent());
            return;
        }
        if (btn == &resetButton) {
            listener.resetSiren(currentSirenId);
            return;
        }

#if COMPOSESIREN_PARK_BRIDGE
        if (btn == &physicalButton) {
            const bool on = physicalButton.getToggleState();
            stAllButton.setEnabled(on);
            listener.physicalSirensSwitched(on);
            refreshPhysicalSirensTooltip();
            return;
        }

        if (btn == &stAllButton) {
            listener.stAllSwitched(stAllButton.getToggleState());
            return;
        }
#endif

        if (btn == &resetAllButton) {
            listener.resetSiren(std::nullopt);
            return;
        }

        if (btn == &selectResourcesButton) {
            const std::string resourcesPath = listener.getResourcesPath();
            fileChooser = std::make_unique<juce::FileChooser>(
                "Select a file", juce::File(resourcesPath), ""
            );

            auto flags =
                juce::FileBrowserComponent::openMode
                | juce::FileBrowserComponent::canSelectDirectories;

            fileChooser->launchAsync(flags,[this](const juce::FileChooser& chooser) {
                // get the result to update resourcesPath
                juce::File newResourcesPath = chooser.getResult();
                listener.selectedNewResourcesPath(
                    juce::File::addTrailingSeparator(
                        newResourcesPath.getFullPathName()
                    ).toStdString()
                );
            });
        }
    }

private:
    Listener& listener;

    std::optional<sirenId> currentSirenId{std::nullopt};
    bool hasResetAllButton{false};
#if COMPOSESIREN_PARK_BRIDGE
    bool hasStAllSwitch{false};
#endif

    juce::TextButton resetButton;

    #if COMPOSESIREN_RECORD

    juce::TextButton recordButton;

    #endif
    juce::TextButton aboutButton;
    juce::TextButton resetAllButton;
    juce::TextButton selectResourcesButton;
#if COMPOSESIREN_PARK_BRIDGE
    juce::ToggleButton physicalButton;
    juce::ToggleButton stAllButton;
#endif

    // the song in the title bar (set_song_title / clear_song_title)
    SongDisplay song;
    juce::String defaultWindowTitle;
    juce::Label songTitle;

    std::unique_ptr<juce::FileChooser> fileChooser;
};

#endif //COMPOSESIREN_RESETCOMPONENT_H
