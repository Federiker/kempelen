use crate::{Error, Result, Voice, assets::SpeechData};
use ndarray::Axis;
use ort::value::Tensor;
use std::{fs::read, path::Path};

#[path = "synth.rs"]
mod synth;

pub use synth::{Device, Synthesizer};

/// Load a voice style vector indexed by the phoneme sequence length.
///
/// Loads verified local voice data and errors when that sequence length
/// is outside the voice's available style vectors.
pub fn load_voice(data: &SpeechData, voice: Voice, sequence_length: usize) -> Result<Tensor<f32>> {
    let path = data.voice(voice)?;
    let styles = synth::parse_styles(&read(&path).map_err(Error::file("read", &path))?)?;
    if sequence_length >= styles.nrows() {
        return Err(Error::NoStyle {
            styles: styles.nrows(),
            length: sequence_length,
        });
    }
    let style = styles.row(sequence_length).to_owned().insert_axis(Axis(0));
    Ok(Tensor::from_array(style)?)
}
/// Synthesize `text` to a mono 24 kHz, 16-bit PCM WAV at `out` using default options.
pub fn tts(data: &SpeechData, text: &str, voice: Voice, out: &Path) -> Result<()> {
    tts_with_options(data, text, voice, out, TtsOptions::default())
}

#[derive(Debug, Clone, Copy)]
/// Runtime settings for speech synthesis.
pub struct TtsOptions {
    /// Playback rate; must be finite and greater than zero.
    pub speed: f32,
}

impl Default for TtsOptions {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

/// Synthesize text to a mono 24 kHz, 16-bit PCM WAV with explicit options.
///
/// Loads verified local model and voice assets and writes the WAV to `out`.
pub fn tts_with_options(
    data: &SpeechData,
    text: &str,
    voice: Voice,
    out: &Path,
    options: TtsOptions,
) -> Result<()> {
    let mut synthesizer = Synthesizer::new(data, voice, options.speed)?;
    synthesizer.synthesize_to_wav(text, out)
}
#[cfg(test)]
#[path = "../../tests/unit/tts/tests.rs"]
mod tests;
