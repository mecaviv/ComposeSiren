//! The recorder: the audio thread copies each block into a ring buffer, a
//! writer thread started by [`Recorder::start`] encodes it into the file, and
//! [`Recorder::stop`] lets it drain the ring and complete the file.

use std::cell::UnsafeCell;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::flac::FlacWriter;

/// Samples the ring holds: about 10 s of stereo at 48 kHz. The writer drains
/// it every few milliseconds; the margin covers a slow disk.
const RING_SAMPLES: usize = 1 << 20;

/// What the file holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Format {
    /// FLAC, 24-bit integer samples (lossless for 24 bits, about half the size).
    Flac24 = 0,
    /// WAV, 24-bit integer samples.
    Wav24 = 1,
    /// WAV, 32-bit float samples: exactly what the DSP produced.
    WavFloat = 2,
}

impl Format {
    /// The format numbered `value` in the C ABI.
    #[must_use]
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Format::Flac24),
            1 => Some(Format::Wav24),
            2 => Some(Format::WavFloat),
            _ => None,
        }
    }

    /// The usual file extension.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Format::Flac24 => "flac",
            Format::Wav24 | Format::WavFloat => "wav",
        }
    }
}

/// Where a recording stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// A recording is running.
    pub recording: bool,
    /// Its file, or the last recording's.
    pub path: Option<PathBuf>,
    /// Its format.
    pub format: Option<Format>,
    /// Its sample rate, in Hz.
    pub sample_rate: u32,
    /// Its channels.
    pub channels: u32,
    /// Frames written to the file so far.
    pub frames_written: u64,
    /// Frames the audio thread could not hand over (the ring was full, or the
    /// block had another channel count).
    pub frames_dropped: u64,
    /// The writer's error, if it stopped on one.
    pub error: Option<String>,
}

/// A finished recording.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// The file.
    pub path: PathBuf,
    /// Frames in the file.
    pub frames_written: u64,
    /// Frames the audio thread could not hand over.
    pub frames_dropped: u64,
    /// Its sample rate, in Hz.
    pub sample_rate: u32,
}

struct Shared {
    recording: AtomicBool,
    channels: AtomicU32,
    written: AtomicU64,
    dropped: AtomicU64,
    error: Mutex<Option<String>>,
}

struct Session {
    thread: JoinHandle<Consumer<f32>>,
    stop: Arc<AtomicBool>,
    path: PathBuf,
    format: Format,
    sample_rate: u32,
}

/// Records the blocks handed to [`Recorder::process`].
///
/// [`Recorder::process`] must be called from one thread at a time (the audio
/// thread); the other methods from any thread.
pub struct Recorder {
    producer: UnsafeCell<Producer<f32>>,
    consumer: Mutex<Option<Consumer<f32>>>,
    shared: Arc<Shared>,
    session: Mutex<Option<Session>>,
    last: Mutex<Option<(PathBuf, Format, u32)>>,
}

// SAFETY: the producer is only touched by `process`, which its contract
// restricts to one thread at a time; everything else is atomics and mutexes.
unsafe impl Sync for Recorder {}
// SAFETY: as above; the ring's halves are `Send`.
unsafe impl Send for Recorder {}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

enum Encoder {
    Flac(Box<FlacWriter>),
    Wav(hound::WavWriter<std::io::BufWriter<File>>, Format),
}

impl Encoder {
    fn create(path: &Path, format: Format, sample_rate: u32, channels: u32) -> Result<Self, String> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        match format {
            Format::Flac24 => FlacWriter::create(file, sample_rate as usize, channels as usize, 24)
                .map(|w| Encoder::Flac(Box::new(w)))
                .map_err(|e| e.to_string()),
            Format::Wav24 | Format::WavFloat => {
                let (bits, sample_format) = if format == Format::Wav24 {
                    (24, hound::SampleFormat::Int)
                } else {
                    (32, hound::SampleFormat::Float)
                };
                let spec = hound::WavSpec { channels: channels as u16, sample_rate, bits_per_sample: bits, sample_format };
                hound::WavWriter::new(std::io::BufWriter::new(file), spec)
                    .map(|w| Encoder::Wav(w, format))
                    .map_err(|e| e.to_string())
            }
        }
    }

    fn push(&mut self, samples: &[f32], scratch: &mut Vec<i32>) -> Result<(), String> {
        match self {
            Encoder::Flac(w) => {
                scratch.clear();
                scratch.extend(samples.iter().map(|&x| to_24(x)));
                w.push(scratch).map_err(|e| e.to_string())
            }
            Encoder::Wav(w, Format::Wav24) => {
                samples.iter().try_for_each(|&x| w.write_sample(to_24(x))).map_err(|e| e.to_string())
            }
            Encoder::Wav(w, _) => samples.iter().try_for_each(|&x| w.write_sample(x)).map_err(|e| e.to_string()),
        }
    }

    fn finish(self) -> Result<(), String> {
        match self {
            Encoder::Flac(w) => w.finish().map_err(|e| e.to_string()),
            Encoder::Wav(w, _) => w.finalize().map_err(|e| e.to_string()),
        }
    }
}

/// A float sample as a 24-bit integer, clipped.
fn to_24(x: f32) -> i32 {
    (x.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32
}

impl Recorder {
    /// A recorder, not recording.
    #[must_use]
    pub fn new() -> Self {
        let (producer, consumer) = RingBuffer::new(RING_SAMPLES);
        Recorder {
            producer: UnsafeCell::new(producer),
            consumer: Mutex::new(Some(consumer)),
            shared: Arc::new(Shared {
                recording: AtomicBool::new(false),
                channels: AtomicU32::new(0),
                written: AtomicU64::new(0),
                dropped: AtomicU64::new(0),
                error: Mutex::new(None),
            }),
            session: Mutex::new(None),
            last: Mutex::new(None),
        }
    }

    /// The audio thread's output block, one slice per channel. Copies it into
    /// the ring when recording; allocates nothing, takes no lock.
    pub fn process(&self, channels: &[&[f32]]) {
        let shared = &*self.shared;
        if !shared.recording.load(Ordering::Acquire) {
            return;
        }
        let frames = channels.iter().map(|c| c.len()).min().unwrap_or(0);
        if frames == 0 {
            return;
        }
        if channels.len() as u32 != shared.channels.load(Ordering::Relaxed) {
            shared.dropped.fetch_add(frames as u64, Ordering::Relaxed);
            return;
        }
        // SAFETY: `process` runs on one thread at a time (its contract).
        let producer = unsafe { &mut *self.producer.get() };
        match producer.write_chunk_uninit(frames * channels.len()) {
            Ok(chunk) => {
                let samples = (0..frames).flat_map(|i| channels.iter().map(move |c| c[i]));
                chunk.fill_from_iter(samples);
            }
            Err(_) => {
                shared.dropped.fetch_add(frames as u64, Ordering::Relaxed);
            }
        }
    }

    /// Starts recording to `path`: blocks of `channels` channels at
    /// `sample_rate`. Fails if a recording is running or the file cannot be
    /// created.
    pub fn start(&self, path: &Path, format: Format, sample_rate: u32, channels: u32) -> Result<(), String> {
        let mut session = self.session.lock().expect("no poisoning");
        if session.is_some() {
            return Err("a recording is already running".into());
        }
        if sample_rate == 0 || channels == 0 {
            return Err("no audio format yet: the device has not started".into());
        }
        let mut encoder = Encoder::create(path, format, sample_rate, channels)?;
        let mut consumer = self.consumer.lock().expect("no poisoning").take().ok_or("the ring is in use")?;
        // Whatever an earlier recording left behind.
        let stale = consumer.slots();
        if let Ok(chunk) = consumer.read_chunk(stale) {
            chunk.commit_all();
        }
        let shared = Arc::clone(&self.shared);
        shared.written.store(0, Ordering::Relaxed);
        shared.dropped.store(0, Ordering::Relaxed);
        *shared.error.lock().expect("no poisoning") = None;
        shared.channels.store(channels, Ordering::Relaxed);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("composesiren-record".into())
            .spawn(move || {
                let nch = channels as usize;
                let mut scratch = Vec::new();
                let mut failed = None;
                loop {
                    let stopping = stop_flag.load(Ordering::Acquire);
                    let available = consumer.slots() / nch * nch;
                    if available > 0 {
                        if let Ok(chunk) = consumer.read_chunk(available) {
                            let (a, b) = chunk.as_slices();
                            for part in [a, b] {
                                if failed.is_none()
                                    && !part.is_empty()
                                    && let Err(e) = encoder.push(part, &mut scratch)
                                {
                                    failed = Some(e);
                                }
                            }
                            chunk.commit_all();
                            shared.written.fetch_add((available / nch) as u64, Ordering::Relaxed);
                        }
                    } else if stopping {
                        break;
                    } else {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
                let result = match failed {
                    Some(e) => Err(e),
                    None => encoder.finish(),
                };
                if let Err(e) = result {
                    *shared.error.lock().expect("no poisoning") = Some(e);
                }
                consumer
            })
            .map_err(|e| e.to_string())?;
        self.shared.recording.store(true, Ordering::Release);
        *self.last.lock().expect("no poisoning") = Some((path.to_path_buf(), format, sample_rate));
        *session = Some(Session { thread, stop, path: path.to_path_buf(), format, sample_rate });
        Ok(())
    }

    /// Stops recording: the writer drains the ring and completes the file.
    pub fn stop(&self) -> Result<Summary, String> {
        let session = self.session.lock().expect("no poisoning").take().ok_or("no recording is running")?;
        self.shared.recording.store(false, Ordering::Release);
        session.stop.store(true, Ordering::Release);
        let consumer = session.thread.join().map_err(|_| "the writer thread panicked".to_owned())?;
        *self.consumer.lock().expect("no poisoning") = Some(consumer);
        if let Some(e) = self.shared.error.lock().expect("no poisoning").clone() {
            return Err(e);
        }
        Ok(Summary {
            path: session.path,
            frames_written: self.shared.written.load(Ordering::Relaxed),
            frames_dropped: self.shared.dropped.load(Ordering::Relaxed),
            sample_rate: session.sample_rate,
        })
    }

    /// The running recording, or the last one.
    #[must_use]
    pub fn status(&self) -> Status {
        let running = self.session.lock().expect("no poisoning").as_ref().map(|s| (s.path.clone(), s.format, s.sample_rate));
        let (path, format, sample_rate) = match running.clone().or_else(|| self.last.lock().expect("no poisoning").clone()) {
            Some((p, f, r)) => (Some(p), Some(f), r),
            None => (None, None, 0),
        };
        Status {
            recording: running.is_some(),
            path,
            format,
            sample_rate,
            channels: self.shared.channels.load(Ordering::Relaxed),
            frames_written: self.shared.written.load(Ordering::Relaxed),
            frames_dropped: self.shared.dropped.load(Ordering::Relaxed),
            error: self.shared.error.lock().expect("no poisoning").clone(),
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
