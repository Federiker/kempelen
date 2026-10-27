//! Loudness normalisation of narration.
//!
//! Kokoro's voices differ in loudness by up to 12 dB, so the volume of a book would change with the
//! voice, and a chapter's heading, subtitle and text would not match. Every synthesized passage is
//! brought to the same speech level instead, then its peaks are limited. The default target
//! satisfies the technical rules of audiobook retailers such as ACX/Audible: overall RMS between
//! -23 and -18 dBFS and no peak above -3 dBFS.

use super::limiter::limit_peaks_to;

/// Samples per second of the model's output.
const SAMPLE_RATE: usize = 24_000;
/// Measurement block: 20 ms.
const BLOCK: usize = SAMPLE_RATE / 50;
/// Blocks quieter than this (pauses, breath, noise floor) do not count as speech: -50 dBFS RMS.
const SPEECH_FLOOR_POWER: f32 = 1.0e-5;
/// Less speech than this (0.3 s) is too little to measure reliably.
const MIN_SPEECH_BLOCKS: usize = 15;
/// The largest cut and boost applied, in dB, so a mis-measurement can never wreck a passage.
const MAX_CUT_DB: f32 = 12.0;
const MAX_BOOST_DB: f32 = 18.0;

/// The level narration is brought to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessTarget {
    /// RMS of the speech itself in dBFS, ignoring pauses.
    pub speech_rms_dbfs: f32,
    /// Highest sample magnitude allowed afterwards (linear, 1.0 is full scale).
    pub peak_ceiling: f32,
}

impl LoudnessTarget {
    /// Speech at -20 dBFS RMS with peaks at or below -3 dBFS (0.708).
    ///
    /// Pauses lower the overall RMS by a few dB, which keeps a finished audiobook inside
    /// ACX's -23 to -18 dBFS window.
    pub const AUDIOBOOK: Self = Self {
        speech_rms_dbfs: -20.0,
        peak_ceiling: 0.707_9,
    };

    /// Like [`Self::AUDIOBOOK`] with peaks at -4 dBFS (0.631): the MP3 encode of a retailer file
    /// can overshoot a little, and the retailer limit is "below -3 dB".
    pub const RETAIL: Self = Self {
        speech_rms_dbfs: -20.0,
        peak_ceiling: 0.631,
    };
}

/// RMS of the speech in `samples` in dBFS, or `None` when there is not enough speech to measure.
///
/// Only 20 ms blocks louder than -50 dBFS count, so pauses do not pull the figure down.
pub fn speech_rms_dbfs(samples: &[f32]) -> Option<f32> {
    let mut meter = SpeechMeter::default();
    meter.push(samples);
    meter.speech_rms_dbfs()
}

/// [`speech_rms_dbfs`] for audio that arrives a piece at a time: the same figure whatever the
/// pieces. Blocks are counted from the first sample; a final partial block is left out.
#[derive(Default)]
pub struct SpeechMeter {
    /// Samples of the current, unfinished block.
    block: Vec<f32>,
    power: f64,
    blocks: usize,
}

impl SpeechMeter {
    #[expect(clippy::cast_precision_loss, reason = "a block is 480 samples")]
    pub fn push(&mut self, mut samples: &[f32]) {
        while !samples.is_empty() {
            let take = (BLOCK - self.block.len()).min(samples.len());
            self.block.extend_from_slice(&samples[..take]);
            samples = &samples[take..];
            if self.block.len() == BLOCK {
                let power = self.block.iter().map(|s| s * s).sum::<f32>() / BLOCK as f32;
                if power > SPEECH_FLOOR_POWER {
                    self.power += f64::from(power);
                    self.blocks += 1;
                }
                self.block.clear();
            }
        }
    }

    /// The speech level so far, or `None` when there is not enough speech to measure.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "a level in dB needs no more than f32"
    )]
    pub fn speech_rms_dbfs(&self) -> Option<f32> {
        (self.blocks >= MIN_SPEECH_BLOCKS)
            .then(|| 10.0 * ((self.power / self.blocks as f64) as f32).log10())
    }

    /// The gain in dB that brings this speech to `target`, or `None` when it cannot be measured.
    pub fn gain_db(&self, target: LoudnessTarget) -> Option<f32> {
        self.speech_rms_dbfs()
            .map(|level| (target.speech_rms_dbfs - level).clamp(-MAX_CUT_DB, MAX_BOOST_DB))
    }
}

/// The linear factor of a gain in dB, as [`normalize_loudness`] applies it.
pub fn linear_gain(gain_db: f32) -> f32 {
    10.0f32.powf(gain_db / 20.0)
}

/// Bring `samples` to `target` and limit their peaks. Returns the gain applied in dB, or `None`
/// when there was too little speech to measure (the peaks are still limited).
pub fn normalize_loudness(samples: &mut [f32], target: LoudnessTarget) -> Option<f32> {
    let mut meter = SpeechMeter::default();
    meter.push(samples);
    let gain_db = meter.gain_db(target);
    if let Some(gain_db) = gain_db {
        let gain = linear_gain(gain_db);
        for sample in samples.iter_mut() {
            *sample *= gain;
        }
    }
    limit_peaks_to(samples, target.peak_ceiling);
    gain_db
}

#[cfg(test)]
#[path = "../../tests/unit/tts/loudness.rs"]
mod tests;
