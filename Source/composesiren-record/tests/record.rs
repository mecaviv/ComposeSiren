//! Records a known signal through the recorder, as an audio thread would hand
//! it over (blocks of changing size, from another thread), and reads the file
//! back with an independent decoder (claxon for FLAC, hound for WAV).

use std::path::PathBuf;
use std::sync::Arc;

use composesiren_record::{Format, Recorder};

const RATE: u32 = 48_000;

fn signal(frames: usize) -> [Vec<f32>; 2] {
    let tone = |f: f32, i: usize| (2.0 * std::f32::consts::PI * f * i as f32 / RATE as f32).sin() * 0.5;
    [(0..frames).map(|i| tone(440.0, i)).collect(), (0..frames).map(|i| tone(660.0, i) * 0.8).collect()]
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
        Format::WavFloat => hound::WavReader::open(&path).unwrap().samples::<f32>().map(Result::unwrap).collect(),
    };
    let _ = std::fs::remove_file(&path);
    (decoded, input, summary.frames_written, summary.frames_dropped)
}

#[test]
fn each_format_reads_back_the_signal() {
    for (format, tolerance) in [(Format::Flac24, 1.0 / 8_388_607.0), (Format::Wav24, 1.0 / 8_388_607.0), (Format::WavFloat, 0.0)] {
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
