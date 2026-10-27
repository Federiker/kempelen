//! The [`Phonemizer`] for Mandarin narration.

use super::reader::{chinese_phonemes_with, to_simplified};
use crate::Result;
use crate::phonemes::{engine::Phonemizer, jespersen::JespersenPhonemizer};

/// Mandarin pronunciation; see the [module](self) documentation.
///
/// Misaki leaves Latin letters as they are, which Kokoro cannot pronounce; narration reads
/// English words with jespersen's American English instead, and leaves out what it cannot read.
pub struct MandarinPhonemizer {
    english: Option<JespersenPhonemizer>,
}

impl Default for MandarinPhonemizer {
    fn default() -> Self {
        Self {
            english: JespersenPhonemizer::new("en-us").ok(),
        }
    }
}

impl MandarinPhonemizer {
    pub fn new() -> Self {
        Self::default()
    }

    fn english(&self, words: &str) -> String {
        self.english
            .as_ref()
            .and_then(|engine| engine.text_to_phonemes(words).ok())
            .unwrap_or_default()
    }
}

impl Phonemizer for MandarinPhonemizer {
    fn engine(&self) -> &'static str {
        "mandarin"
    }

    fn language(&self) -> &'static str {
        "cmn"
    }

    fn text_to_phonemes(&self, words: &str) -> Result<String> {
        // Traditional text (Taiwan, Hong Kong) is read as the same words in Simplified characters.
        let spoken = chinese_phonemes_with(&to_simplified(words), |latin| {
            format!(" {} ", self.english(latin))
        });
        Ok(spoken.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}
