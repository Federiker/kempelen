//! Mono 24 kHz, 16-bit PCM WAV files, the model's output format.

use crate::{Error, Result};
use std::{
    fs::File,
    io::{BufWriter, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

/// Samples per second of the model's output.
pub const SAMPLE_RATE: u32 = 24_000;

/// Length of the RIFF/WAVE header that precedes the samples.
const HEADER_LEN: usize = 44;

/// The header of a WAV holding `samples` samples; errors when it is too long for the format.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the RIFF length check bounds the data length"
)]
fn header(samples: usize) -> Result<[u8; HEADER_LEN]> {
    let data_len = samples.checked_mul(2).ok_or(Error::WavTooLarge)?;
    let riff_len = u32::try_from(36usize.checked_add(data_len).ok_or(Error::WavTooLarge)?)
        .map_err(|_| Error::WavTooLarge)?;
    let mut bytes = [0; HEADER_LEN];
    let mut at = 0;
    for part in [
        b"RIFF".as_slice(),
        &riff_len.to_le_bytes(),
        b"WAVEfmt ",
        &16u32.to_le_bytes(),
        &1u16.to_le_bytes(),
        &1u16.to_le_bytes(),
        &SAMPLE_RATE.to_le_bytes(),
        &(SAMPLE_RATE * 2).to_le_bytes(),
        &2u16.to_le_bytes(),
        &16u16.to_le_bytes(),
        b"data",
        &(data_len as u32).to_le_bytes(),
    ] {
        bytes[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    }
    Ok(bytes)
}

/// A sample as written to the file: clamped to full scale and truncated to 16 bits.
#[expect(
    clippy::cast_possible_truncation,
    reason = "clamped to full scale first; truncation is the format"
)]
fn pcm16(sample: f32) -> [u8; 2] {
    ((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes()
}

/// Encode 24 kHz mono samples as a 16-bit PCM WAV file.
pub fn wav_bytes(waveform: &[f32]) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + waveform.len() * 2);
    bytes.extend_from_slice(&header(waveform.len())?);
    for &sample in waveform {
        bytes.extend_from_slice(&pcm16(sample));
    }
    Ok(bytes)
}

/// Writes a WAV a piece at a time; the header gets its lengths when the file is finished.
/// The bytes are those [`wav_bytes`] gives for the same samples.
pub struct WavWriter {
    file: BufWriter<File>,
    path: PathBuf,
    samples: usize,
}

impl WavWriter {
    pub fn create(path: &Path) -> Result<Self> {
        let mut file = BufWriter::new(File::create(path).map_err(Error::file("create", path))?);
        file.write_all(&header(0)?)
            .map_err(Error::file("write", path))?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            samples: 0,
        })
    }

    pub fn write(&mut self, samples: &[f32]) -> Result<()> {
        for &sample in samples {
            self.file
                .write_all(&pcm16(sample))
                .map_err(Error::file("write", &self.path))?;
        }
        self.samples += samples.len();
        Ok(())
    }

    /// Append `samples` of digital silence.
    pub fn silence(&mut self, samples: usize) -> Result<()> {
        for _ in 0..samples {
            self.file
                .write_all(&[0, 0])
                .map_err(Error::file("write", &self.path))?;
        }
        self.samples += samples;
        Ok(())
    }

    /// Samples written so far.
    pub fn len(&self) -> usize {
        self.samples
    }

    pub fn is_empty(&self) -> bool {
        self.samples == 0
    }

    /// Write the header's lengths and close the file.
    pub fn finish(mut self) -> Result<()> {
        let header = header(self.samples)?;
        let written = (|| {
            self.file.seek(SeekFrom::Start(0))?;
            self.file.write_all(&header)?;
            self.file.flush()
        })();
        written.map_err(Error::file("write", &self.path))
    }
}

/// Samples of silence that make `seconds` at the model's rate, rounded to the nearest sample,
/// as FFmpeg's `apad` with `-t` used to pad (measured on 37 lengths and 13 paddings).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "not negative, and any pause is far shorter than usize::MAX samples"
)]
pub fn silence_samples(seconds: f64) -> usize {
    (seconds.max(0.0) * f64::from(SAMPLE_RATE)).round() as usize
}

#[cfg(test)]
#[path = "../../tests/unit/tts/wav.rs"]
mod tests;
