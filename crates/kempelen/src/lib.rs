//! Local speech: Kokoro ONNX synthesis with jespersen's pronunciation.
//!
//! A [`tts::Synthesizer`] loads the pinned Kokoro v1.0 model and a [`Voice`] from the caller's
//! [`assets::SpeechData`] (checked by size and SHA-256, never downloaded), pronounces the text as its
//! book [`Language`] with [`phonemes`] (jespersen, or the Mandarin front end), and gives 24 kHz mono
//! samples or a 16-bit WAV, levelled and peak-limited. See README.md.

pub mod assets;
pub mod cuda;
mod error;
#[cfg(feature = "gpu")]
pub mod gpu;
mod language;
pub mod phonemes;
pub mod tts;
mod voice;

pub use error::{AssetProblem, Error, Result};
pub use language::Language;
pub use voice::Voice;

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
