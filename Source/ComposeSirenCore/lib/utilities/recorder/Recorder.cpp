#include "Recorder.h"

#if COMPOSESIREN_RECORD

#include "composesiren_record.h"

Recorder::Recorder() : rec(cs_rec_create()) {}

Recorder::~Recorder()
{
    cs_rec_destroy(rec);
}

void Recorder::process(const juce::AudioBuffer<float>& audio) noexcept
{
    const int wanted = channels.load(std::memory_order_relaxed);
    const int n = juce::jmin(wanted, audio.getNumChannels());
    if (n <= 0)
        return;
    cs_rec_process(rec, audio.getArrayOfReadPointers(), static_cast<uint32_t>(n),
                   static_cast<uint32_t>(audio.getNumSamples()));
}

void Recorder::setAudioFormat(double newRate, int newChannels)
{
    const auto current = status();
    const bool otherRate = juce::roundToInt(current.sampleRate) != juce::roundToInt(newRate);
    if (current.recording && (otherRate || current.channels != newChannels))
        stop();
    sampleRate.store(newRate);
    channels.store(newChannels);
}

juce::Result Recorder::start(const juce::File& file, Format format)
{
    char error[256] = {};
    const auto rate = sampleRate.load();
    const auto n = channels.load();
    if (cs_rec_start(rec, file.getFullPathName().toRawUTF8(), static_cast<uint32_t>(format),
                     static_cast<uint32_t>(juce::roundToInt(rate)), static_cast<uint32_t>(n),
                     error, sizeof(error)) != 0)
        return juce::Result::fail(juce::String::fromUTF8(error));
    return juce::Result::ok();
}

juce::Result Recorder::stop()
{
    char error[256] = {};
    if (cs_rec_stop(rec, error, sizeof(error)) != 0)
        return juce::Result::fail(juce::String::fromUTF8(error));
    return juce::Result::ok();
}

Recorder::Status Recorder::status() const
{
    cs_rec_status_t raw {};
    cs_rec_status(rec, &raw);
    Status s;
    s.recording = raw.recording != 0;
    if (raw.format >= 0)
        s.format = static_cast<Format>(raw.format);
    s.sampleRate = raw.sample_rate;
    s.channels = static_cast<int>(raw.channels);
    s.framesWritten = static_cast<juce::int64>(raw.frames_written);
    s.framesDropped = static_cast<juce::int64>(raw.frames_dropped);
    char text[1024] = {};
    if (cs_rec_path(rec, text, sizeof(text)) > 0)
        s.file = juce::File(juce::String::fromUTF8(text));
    if (raw.failed != 0 && cs_rec_error(rec, text, sizeof(text)) > 0)
        s.error = juce::String::fromUTF8(text);
    return s;
}

juce::String Recorder::extension(Format format)
{
    return format == Format::flac24 ? "flac" : "wav";
}

juce::String Recorder::name(Format format)
{
    switch (format) {
        case Format::flac24: return "flac";
        case Format::wav24: return "wav";
        case Format::wavFloat: return "wav-float";
    }
    return "flac";
}

std::optional<Recorder::Format> Recorder::formatNamed(const juce::String& text)
{
    for (auto f : { Format::flac24, Format::wav24, Format::wavFloat })
        if (text.equalsIgnoreCase(name(f)))
            return f;
    return std::nullopt;
}

juce::File Recorder::defaultFile(Format format)
{
    const auto stamp = juce::Time::getCurrentTime().formatted("%Y%m%d-%H%M%S");
    return juce::File::getSpecialLocation(juce::File::userMusicDirectory)
        .getChildFile("ComposeSiren")
        .getChildFile("ComposeSiren-" + stamp + "." + extension(format));
}

#endif
