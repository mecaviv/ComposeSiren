#pragma once

// Records the audio output to FLAC or WAV (COMPOSESIREN_RECORD). The encoders
// are the Rust crate Source/composesiren-record: process() only copies the
// block into a lock-free ring; a writer thread encodes and writes the file.

#include <atomic>
#include <optional>

#include <juce_audio_basics/juce_audio_basics.h>
#include <juce_core/juce_core.h>

#ifndef COMPOSESIREN_RECORD
#define COMPOSESIREN_RECORD 0
#endif

#if COMPOSESIREN_RECORD

struct cs_rec_t;

class Recorder
{
public:
    enum class Format { flac24 = 0, wav24 = 1, wavFloat = 2 };

    struct Status
    {
        bool recording = false;
        std::optional<Format> format;
        double sampleRate = 0;
        int channels = 0;
        juce::int64 framesWritten = 0;
        juce::int64 framesDropped = 0;
        juce::File file;
        juce::String error;

        double seconds() const { return sampleRate > 0 ? static_cast<double>(framesWritten) / sampleRate : 0; }
    };

    Recorder();
    ~Recorder();

    Recorder(const Recorder&) = delete;
    Recorder& operator=(const Recorder&) = delete;

    // The audio thread's output block. Allocates nothing, takes no lock.
    void process(const juce::AudioBuffer<float>& audio) noexcept;

    // From prepareToPlay: the format the next blocks have. A recording at
    // another rate or channel count is stopped (its file completed).
    void setAudioFormat(double sampleRate, int channels);

    juce::Result start(const juce::File& file, Format format);
    juce::Result stop();
    Status status() const;

    // ~/Music/ComposeSiren/ComposeSiren-<date>-<time>.<ext>
    static juce::File defaultFile(Format format);
    static juce::String extension(Format format);
    static juce::String name(Format format);
    static std::optional<Format> formatNamed(const juce::String& name);

private:
    cs_rec_t* rec = nullptr;
    std::atomic<double> sampleRate { 0 };
    std::atomic<int> channels { 0 };
};

#endif
