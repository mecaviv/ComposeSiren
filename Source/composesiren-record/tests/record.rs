//! Records a known signal through the recorder, as an audio thread would hand
//! it over (blocks of changing size, from another thread), and reads the file
//! back with an independent decoder (claxon for FLAC, hound for WAV).

use std::path::PathBuf;
use std::sync::Arc;

use composesiren_record::{Format, Recorder};

const RATE: u32 = 48_000;

fn signal(frames: usize) -> [Vec<f32>; 2] {
    let tone =
        |f: f32, i: usize| (2.0 * std::f32::consts::PI * f * i as f32 / RATE as f32).sin() * 0.5;
    [
        (0..frames).map(|i| tone(440.0, i)).collect(),
        (0..frames).map(|i| tone(660.0, i) * 0.8).collect(),
    ]
}

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("composesiren-record-{}-{name}", std::process::id()))
}

/// Records `seconds` of the signal in `format`; returns what the decoder read,
/// interleaved, as floats.
fn record(format: Format, seconds: usize) -> (Vec<f32>, [Vec<f32>; 2], u64, u64) {
    let frames = RATE as usize * seconds;
    let input = signal(frames);
    let path = temp(&format!("{format:?}.{}", format.extension()));
    let recorder = Arc::new(Recorder::new());
    recorder.start(&path, format, RATE, 2).unwrap();
    let audio = {
        let recorder = Arc::clone(&recorder);
        let input = input.clone();
        std::thread::spawn(move || {
            let (mut at, mut k) = (0usize, 0usize);
            while at < frames {
                let n = [64, 128, 480, 512, 1024][k % 5].min(frames - at);
                recorder.process(&[&input[0][at..at + n], &input[1][at..at + n]]);
                at += n;
                k += 1;
                if k % 50 == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        })
    };
    audio.join().unwrap();
    let summary = recorder.stop().unwrap();
    let decoded: Vec<f32> = match format {
        Format::Flac24 => claxon::FlacReader::open(&path)
            .unwrap()
            .samples()
            .map(|s| s.unwrap() as f32 / 8_388_607.0)
            .collect(),
        Format::Wav24 => hound::WavReader::open(&path)
            .unwrap()
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / 8_388_607.0)
            .collect(),
        Format::WavFloat => hound::WavReader::open(&path)
            .unwrap()
            .samples::<f32>()
            .map(Result::unwrap)
            .collect(),
    };
    let _ = std::fs::remove_file(&path);
    (
        decoded,
        input,
        summary.frames_written,
        summary.frames_dropped,
    )
}

#[test]
fn each_format_reads_back_the_signal() {
    for (format, tolerance) in [
        (Format::Flac24, 1.0 / 8_388_607.0),
        (Format::Wav24, 1.0 / 8_388_607.0),
        (Format::WavFloat, 0.0),
    ] {
        let (decoded, input, written, dropped) = record(format, 3);
        assert_eq!(dropped, 0, "{format:?}");
        assert_eq!(written, input[0].len() as u64, "{format:?}");
        assert_eq!(decoded.len(), input[0].len() * 2, "{format:?}");
        let worst = decoded
            .chunks(2)
            .zip(input[0].iter().zip(&input[1]))
            .map(|(d, (l, r))| (d[0] - l).abs().max((d[1] - r).abs()))
            .fold(0.0f32, f32::max);
        assert!(worst <= tolerance, "{format:?}: off by {worst}");
    }
}

#[test]
fn blocks_of_another_channel_count_are_dropped_not_recorded() {
    let path = temp("mono.flac");
    let recorder = Recorder::new();
    recorder.start(&path, Format::Flac24, RATE, 2).unwrap();
    let block = vec![0.25f32; 256];
    recorder.process(&[&block]);
    recorder.process(&[&block, &block]);
    let summary = recorder.stop().unwrap();
    assert_eq!((summary.frames_written, summary.frames_dropped), (256, 256));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn nothing_is_kept_between_recordings() {
    let recorder = Recorder::new();
    let block = vec![0.5f32; 512];
    // Not recording: ignored.
    recorder.process(&[&block, &block]);
    let path = temp("second.wav");
    recorder.start(&path, Format::WavFloat, RATE, 2).unwrap();
    recorder.process(&[&block, &block]);
    let summary = recorder.stop().unwrap();
    assert_eq!(summary.frames_written, 512);
    assert!(recorder.stop().is_err(), "nothing to stop");
    assert!(!recorder.status().recording);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_second_start_is_refused_while_recording() {
    let recorder = Recorder::new();
    let (a, b) = (temp("a.flac"), temp("b.flac"));
    recorder.start(&a, Format::Flac24, RATE, 2).unwrap();
    assert!(recorder.start(&b, Format::Flac24, RATE, 2).is_err());
    recorder.stop().unwrap();
    let _ = std::fs::remove_file(&a);
}

/// Feeds 512-frame blocks from an "audio thread" until the recording ends by
/// itself; `level(frame)` gives the sample of both channels.
fn feed_until_it_ends(
    recorder: &Arc<Recorder>,
    level: impl Fn(usize) -> f32 + Send + 'static,
) -> std::thread::JoinHandle<()> {
    let recorder = Arc::clone(recorder);
    std::thread::spawn(move || {
        let mut at = 0usize;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < deadline {
            let block: Vec<f32> = (at..at + 512).map(&level).collect();
            recorder.process(&[&block, &block]);
            at += 512;
            // About real time: 512 frames at 48 kHz.
            std::thread::sleep(std::time::Duration::from_micros(10_600));
            if at > RATE as usize && !recorder.status().recording {
                break;
            }
        }
    })
}

fn decode_flac(path: &std::path::Path) -> Vec<f32> {
    claxon::FlacReader::open(path)
        .unwrap()
        .samples()
        .step_by(2)
        .map(|s| s.unwrap() as f32 / 8_388_607.0)
        .collect()
}

#[test]
fn a_drone_fades_out_after_the_wait() {
    use composesiren_record::Fade;
    use std::time::Duration;
    let path = temp("drone.flac");
    let recorder = Arc::new(Recorder::new());
    recorder.start(&path, Format::Flac24, RATE, 2).unwrap();
    let tone = |i: usize| (2.0 * std::f32::consts::PI * 220.0 * i as f32 / RATE as f32).sin() * 0.5;
    let audio = feed_until_it_ends(&recorder, tone);
    std::thread::sleep(Duration::from_millis(500));
    let before = recorder.status().frames_written;
    recorder
        .stop_fading(Fade {
            wait: Duration::from_millis(300),
            length: Duration::from_millis(400),
        })
        .unwrap();
    assert!(recorder.status().fading);
    audio.join().unwrap();
    let status = recorder.status();
    assert!(!status.recording && !status.fading, "it ends by itself");
    let left = decode_flac(&path);
    let added = left.len() as u64 - before;
    // The wait, then the fade: 0.7 s after the request (a block of slack).
    let expected = (0.7 * f64::from(RATE)) as u64;
    assert!(
        added.abs_diff(expected) <= 1024 + (0.02 * f64::from(RATE)) as u64,
        "{added} frames after the request, {expected} expected"
    );
    let peak = |s: &[f32]| s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    let n = left.len();
    let rate = RATE as usize;
    assert!(
        peak(&left[n - rate / 10 - rate / 10..n - rate / 10]) < 0.3,
        "the fade is under way near the end"
    );
    assert!(peak(&left[n - rate / 100..]) < 0.01, "it ends near silence");
    assert!(
        peak(&left[..rate / 4]) > 0.45,
        "the tone before the fade is untouched"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_sound_that_dies_out_ends_the_recording_early() {
    use composesiren_record::Fade;
    use std::time::Duration;
    let path = temp("decay.flac");
    let recorder = Arc::new(Recorder::new());
    recorder.start(&path, Format::Flac24, RATE, 2).unwrap();
    // A second of tone, then silence.
    let audio = feed_until_it_ends(&recorder, |i| if i < RATE as usize { 0.5 } else { 0.0 });
    std::thread::sleep(Duration::from_millis(1300));
    recorder
        .stop_fading(Fade {
            wait: Duration::from_secs(5),
            length: Duration::from_secs(3),
        })
        .unwrap();
    audio.join().unwrap();
    assert!(!recorder.status().recording);
    let left = decode_flac(&path);
    // Ended a quarter second of silence after the request, not 5 s later.
    let seconds = left.len() as f64 / f64::from(RATE);
    assert!(seconds < 2.2, "{seconds} s: it should end at the silence");
    let _ = std::fs::remove_file(&path);
}
