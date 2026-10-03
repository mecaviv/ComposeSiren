//
// Created by joseph larralde on 21/03/2026.
//

#ifndef COMPOSESIREN_RESETCOMPONENT_H
#define COMPOSESIREN_RESETCOMPONENT_H

#include <juce_gui_basics/juce_gui_basics.h>
#include "../lib/definitions/palette.h"
#include "../lib/definitions/sirenProperties.h"

#ifndef COMPOSESIREN_PARK_BRIDGE
#define COMPOSESIREN_PARK_BRIDGE 0
#endif
#ifndef COMPOSESIREN_SETTINGS
#define COMPOSESIREN_SETTINGS 0
#endif
#include "../lib/utilities/recorder/RecordDialog.h"
#if COMPOSESIREN_SETTINGS
#include "../lib/settings/SettingsDialog.h"
#endif

class MainButtonsComponent : public juce::Component,
                             public juce::TextButton::Listener
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
        // Secondary actions live in one Menu so the 30px top bar stays readable
        // when Settings, Record, About and Reset All are all present.
        menuButton.setColour(juce::TextButton::buttonColourId, juce::Colour { 0xff37474f });
        menuButton.setColour(juce::TextButton::textColourOffId, juce::Colours::whitesmoke);
        menuButton.setButtonText("Menu");
        menuButton.setTooltip("Settings, Record, resources directory, About");
        menuButton.onClick = [this] { showMenu(); };
        addAndMakeVisible(menuButton);

        resetButton.setColour(juce::TextButton::buttonColourId, juce::Colours::darkred);
        resetButton.setColour(juce::TextButton::textColourOffId , juce::Colours::whitesmoke);
        resetButton.setButtonText ("Reset");
        resetButton.addListener(this);
        addAndMakeVisible(resetButton);

        if (hasResetAllButton) {
            resetAllButton.setColour(juce::TextButton::buttonColourId, juce::Colours::darkred);
            resetAllButton.setColour(juce::TextButton::textColourOffId , juce::Colours::whitesmoke);
            resetAllButton.setButtonText ("Reset All");
            resetAllButton.addListener(this);
            addAndMakeVisible(resetAllButton);
        }

        selectResourcesButton.setButtonText("Set resources directory");
        selectResourcesButton.addListener(this);

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
    }

    ~MainButtonsComponent() override = default;

    void paint(juce::Graphics& g) override
    {
        juce::ignoreUnused(g);
    }

    void resized() override
    {
        constexpr float margin = 5;
        auto bounds = getLocalBounds().reduced(static_cast<int>(margin));

        juce::FlexBox fb;
        fb.flexDirection = juce::FlexBox::Direction::row;
        fb.flexWrap = juce::FlexBox::Wrap::noWrap;
        fb.alignItems = juce::FlexBox::AlignItems::flexEnd;
        fb.alignContent = juce::FlexBox::AlignContent::flexEnd;
        fb.justifyContent = juce::FlexBox::JustifyContent::flexEnd;

        const float btnsHeight = static_cast<float>(bounds.getHeight());

        auto add = [&](juce::Button& b, float minW) {
            juce::FlexItem item = juce::FlexItem(b).withMinWidth(minW)
                                                   .withMinHeight(btnsHeight)
                                                   .withFlex(0, 0);
            item.margin = juce::FlexItem::Margin(0.f, 0.f, 0.f, margin);
            fb.items.add(item);
        };

#if COMPOSESIREN_PARK_BRIDGE
        if (hasStAllSwitch) {
            add(physicalButton, 130.f);
            add(stAllButton, 44.f);
        }
#endif
        if (hasResetAllButton)
            add(resetAllButton, 88.f);
        add(resetButton, 70.f);
        add(menuButton, 64.f);

        fb.performLayout(bounds);
    }

    void setSirenIdToReset(std::optional<sirenId> id)
    {
        currentSirenId = id;
    }

#if COMPOSESIREN_PARK_BRIDGE
    void refreshPhysicalSirensTooltip()
    {
        if (hasStAllSwitch)
            physicalButton.setTooltip(listener.physicalSirensTooltip());
    }
#endif

    void buttonClicked(juce::Button* btn) override
    {
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
                juce::File newResourcesPath = chooser.getResult();
                listener.selectedNewResourcesPath(
                    juce::File::addTrailingSeparator(
                        newResourcesPath.getFullPathName()
                    ).toStdString()
                );
            });
        }
    }

    void showMenu()
    {
        juce::PopupMenu menu;
#if COMPOSESIREN_SETTINGS
        menu.addItem("Settings...", [this] { cs::SettingsDialog::show(getTopLevelComponent()); });
#endif
#if COMPOSESIREN_RECORD
        if (listener.getRecorder() != nullptr)
            menu.addItem("Record...", [this] {
                if (auto* recorder = listener.getRecorder())
                    RecordDialog::show(*recorder, getTopLevelComponent());
            });
#endif
#if COMPOSESIREN_DEV_BUILD
        menu.addItem("Set resources directory...", [this] { buttonClicked(&selectResourcesButton); });
#endif
        if (listener.hasAbout())
            menu.addItem("About...", [this] { listener.showAbout(getTopLevelComponent()); });
        menu.showMenuAsync(juce::PopupMenu::Options().withTargetComponent(&menuButton));
    }

private:
    Listener& listener;

    std::optional<sirenId> currentSirenId{std::nullopt};
    bool hasResetAllButton{false};
#if COMPOSESIREN_PARK_BRIDGE
    bool hasStAllSwitch{false};
#endif

    juce::TextButton menuButton;
    juce::TextButton resetButton;
    juce::TextButton resetAllButton;
    juce::TextButton selectResourcesButton;
#if COMPOSESIREN_PARK_BRIDGE
    juce::ToggleButton physicalButton;
    juce::ToggleButton stAllButton;
#endif

    std::unique_ptr<juce::FileChooser> fileChooser;
};

#endif //COMPOSESIREN_RESETCOMPONENT_H
