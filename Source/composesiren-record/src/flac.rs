//! A streaming FLAC writer on `flacenc`: samples come in blocks of any size,
//! each full frame is encoded and appended as it fills, and the header is
//! rewritten on [`FlacWriter::finish`] with the sample count and the MD5. A
//! file cut short (a crash before `finish`) still decodes up to its last
//! complete frame.

use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};

use flacenc::bitsink::MemSink;
use flacenc::component::{BitRepr, Stream};
use flacenc::config;
use flacenc::error::{Verified, Verify};
use flacenc::source::{Context, Fill, FrameBuf};

/// Samples per FLAC frame (the reference encoder's default).
pub const BLOCK_SIZE: usize = 4096;

/// Writes a FLAC file frame by frame (see the module).
pub struct FlacWriter {
    file: BufWriter<File>,
    config: Verified<config::Encoder>,
    header: Stream,
    buf: (FrameBuf, Context),
    channels: usize,
    pending: Vec<i32>,
}

fn other(e: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("{e:?}"))
}

impl FlacWriter {
    /// Creates the file and writes a provisional header.
    pub fn create(file: File, sample_rate: usize, channels: usize, bits: usize) -> io::Result<Self> {
        let mut header = Stream::new(sample_rate, channels, bits).map_err(other)?;
        header.stream_info_mut().set_block_sizes(BLOCK_SIZE, BLOCK_SIZE).map_err(other)?;
        let mut w = FlacWriter {
            file: BufWriter::new(file),
            config: config::Encoder::default().into_verified().map_err(|(_, e)| other(e))?,
            header,
            buf: (FrameBuf::with_size(channels, BLOCK_SIZE).map_err(other)?, Context::new(bits, channels)),
            channels,
            pending: Vec::with_capacity(BLOCK_SIZE * channels * 2),
        };
        w.write_header()?;
        Ok(w)
    }

    fn write_header(&mut self) -> io::Result<()> {
        let mut sink = MemSink::<u8>::new();
        self.header.write(&mut sink).map_err(other)?;
        self.file.write_all(sink.as_slice())
    }

    fn encode(&mut self, frames: usize) -> io::Result<()> {
        let chunk: Vec<i32> = self.pending.drain(..frames * self.channels).collect();
        self.buf.fill_interleaved(&chunk).map_err(other)?;
        let number = self.buf.1.current_frame_number().unwrap_or(0);
        let frame = flacenc::encode_fixed_size_frame(&self.config, &self.buf.0, number, self.header.stream_info())
            .map_err(other)?;
        self.header.stream_info_mut().update_frame_info(&frame);
        let mut sink = MemSink::<u8>::new();
        frame.write(&mut sink).map_err(other)?;
        self.file.write_all(sink.as_slice())
    }

    /// Interleaved samples, a multiple of the channels, in any amount.
    pub fn push(&mut self, samples: &[i32]) -> io::Result<()> {
        self.pending.extend_from_slice(samples);
        while self.pending.len() >= BLOCK_SIZE * self.channels {
            self.encode(BLOCK_SIZE)?;
        }
        Ok(())
    }

    /// Encodes the last, shorter frame and rewrites the header.
    pub fn finish(mut self) -> io::Result<()> {
        let rest = self.pending.len() / self.channels;
        if rest > 0 {
            self.encode(rest)?;
        }
        let (digest, total) = (self.buf.1.md5_digest(), self.buf.1.total_samples());
        let info = self.header.stream_info_mut();
        info.set_md5_digest(&digest);
        info.set_total_samples(total);
        // The spec leaves the last block out of the minimum block size: a
        // fixed-block stream keeps min = max (Apple's decoder otherwise takes
        // it for a variable-block stream and fails).
        info.set_block_sizes(BLOCK_SIZE, BLOCK_SIZE).map_err(other)?;
        self.file.flush()?;
        self.file.seek(SeekFrom::Start(0))?;
        self.write_header()?;
        self.file.flush()
    }
}
