// ProcessorCallbacks.h
#pragma once

#include <functional>
#include <string>
#include "PluginProcessor.h"

struct ProcessorCallbacks
{
    // Global gain
    std::function<float()> getGlobalGain;
    std::function<void(int)> setGlobalGainCC;

    // Resources
    std::function<std::string()> getResourcesPath;

    // Reset
    std::function<void()> resetAllSirens;

    // Reverb getters
    std::function<bool()> isReverbEnabled;
    std::function<float()> getReverbRoomSize;
    std::function<float()> getReverbWet;
    std::function<float()> getReverbDry;
    std::function<float()> getReverbDamp;
    std::function<float()> getReverbWidth;
    std::function<float()> getReverbHighpass;
    std::function<float()> getReverbLowpass;

    // Reverb setters
    std::function<void(float)> setReverbRoomSize;
    std::function<void(float)> setReverbWet;
    std::function<void(float)> setReverbDry;
    std::function<void(float)> setReverbDamp;
    std::function<void(float)> setReverbWidth;
    std::function<void(float)> setReverbHighpass;
    std::function<void(float)> setReverbLowpass;
    std::function<void(bool)> setReverbEnabled;

    // Per-siren volume/pan
    std::function<float(int)> getMasterVolume;
    std::function<void(int,float)> setMasterVolume;
    std::function<float(int)> getPan; // returns 0..1
    std::function<void(int,float)> setPan;

    ProcessorCallbacks() = default;

    // Construct callbacks from a processor
    ProcessorCallbacks(SirenePlugAudioProcessor& p)
    {
        getGlobalGain = [&p]() { return p.mySynth->getGlobalGain(); };
        setGlobalGainCC = [&p](int cc) { p.mySynth->setGlobalGain(cc); };

        getResourcesPath = [&p]() { return p.mySynth->getResourcesPath(); };

        resetAllSirens = [&p]() {
            for (int i = 1; i <= 7; ++i) p.myMidiInHandler->resetSireneCh(i);
        };

        isReverbEnabled = [&p]() { return p.mySynth->isReverbEnabled(); };
        getReverbRoomSize = [&p]() { return p.mySynth->reverb.getroomsize(); };
        getReverbWet = [&p]() { return p.mySynth->reverb.getwet(); };
        getReverbDry = [&p]() { return p.mySynth->reverb.getdry(); };
        getReverbDamp = [&p]() { return p.mySynth->reverb.getdamp(); };
        getReverbWidth = [&p]() { return p.mySynth->reverb.getwidth(); };
        getReverbHighpass = [&p]() { return p.mySynth->getReverbHighpass(); };
        getReverbLowpass = [&p]() { return p.mySynth->getReverbLowpass(); };

        setReverbRoomSize = [&p](float v) { p.mySynth->reverb.setroomsize(v); };
        setReverbWet = [&p](float v) { p.mySynth->reverb.setwet(v); };
        setReverbDry = [&p](float v) { p.mySynth->reverb.setdry(v); };
        setReverbDamp = [&p](float v) { p.mySynth->reverb.setdamp(v); };
        setReverbWidth = [&p](float v) { p.mySynth->reverb.setwidth(v); };
        setReverbHighpass = [&p](float v) { p.mySynth->setReverbHighpass(v); };
        setReverbLowpass = [&p](float v) { p.mySynth->setReverbLowpass(v); };
        setReverbEnabled = [&p](bool e) { p.mySynth->setReverbEnabled(e); };

        getMasterVolume = [&p](int s) { return p.mySynth->getMasterVolume(s); };
        setMasterVolume = [&p](int s, float v) { p.mySynth->setMasterVolume(s, v); };
        getPan = [&p](int s) { return p.mySynth->getPan(s); };
        setPan = [&p](int s, float v) { p.mySynth->setPan(s, v); };
    }
};
