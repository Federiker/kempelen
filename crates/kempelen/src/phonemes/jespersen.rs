//! jespersen: the public-domain text-to-phonemes library that reads every narrated language but
//! Mandarin (docs/jespersen.md).

use super::engine::Phonemizer;
use crate::{Error, Result};
use jespersen::{
    BathVowel, Language, Notation, PortugueseVariety, Pronunciation, Script, SpanishVariety,
    WriteOptions,
};
use std::{collections::BTreeSet, sync::Mutex};

/// jespersen reading one language, in eSpeak NG's notation, which Kokoro was trained on.
///
/// A word jespersen cannot read is retried as American English when it is written in Latin
/// letters (a name inside Russian text); what is still unreadable (symbols, emoji, another
/// script) is left silent and logged once, so one stray character never stops an audiobook.
pub struct JespersenPhonemizer {
    language: Language,
    code: &'static str,
    /// Words already reported as unreadable, so each is logged once per book.
    skipped: Mutex<BTreeSet<String>>,
}

impl JespersenPhonemizer {
    /// The reader for the reading code `code` (`en-us`, `de`, `sr-latn`; see
    /// `reading_code`). Errors for a code jespersen does not read, Mandarin (`cmn`) included.
    pub fn new(code: &str) -> Result<Self> {
        let (code, language) = LANGUAGES
            .iter()
            .find(|(known, _)| *known == code)
            .copied()
            .ok_or_else(|| Error::UnsupportedReading(code.to_owned()))?;
        Ok(Self {
            language,
            code,
            skipped: Mutex::default(),
        })
    }

    /// The words of `text` read as American English, when every part of it can be read so.
    fn as_english(text: &str) -> Option<Vec<Pronunciation>> {
        let sentences = jespersen::phonemize(text, Language::AmericanEnglish).ok()?;
        let mut words = Vec::new();
        for token in sentences.into_iter().flat_map(|sentence| sentence.tokens) {
            words.extend(token.words?);
        }
        Some(words)
    }
}

impl Phonemizer for JespersenPhonemizer {
    fn engine(&self) -> &'static str {
        "jespersen"
    }

    fn language(&self) -> &str {
        self.code
    }

    fn text_to_phonemes(&self, words: &str) -> Result<String> {
        let mut sentences =
            jespersen::phonemize(words, self.language).map_err(|source| Error::Phonemes {
                text: words.to_owned(),
                writing: false,
                source,
            })?;
        let mut unreadable = Vec::new();
        for token in sentences
            .iter_mut()
            .flat_map(|sentence| &mut sentence.tokens)
        {
            if token.words.is_some() {
                continue;
            }
            let english = (token.script == Script::Latin
                && self.language != Language::AmericanEnglish)
                .then(|| Self::as_english(token.text))
                .flatten();
            if english.is_none() {
                unreadable.push(token.text.to_owned());
            }
            token.words = Some(english.unwrap_or_default());
        }
        if !unreadable.is_empty() {
            let mut skipped = self
                .skipped
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let new: Vec<String> = unreadable
                .into_iter()
                .filter(|word| skipped.insert(word.clone()))
                .collect();
            if !new.is_empty() {
                log::warn!(
                    "Narration ({}) leaves out what it cannot pronounce: {}",
                    self.code,
                    new.join(" ")
                );
            }
        }
        // The caller keeps the punctuation Kokoro uses; what reaches here (an abbreviation's or an
        // initial's full stop, hyphens, slashes) must not become a pause mid-sentence.
        let options = WriteOptions {
            notation: Notation::Espeak,
            drop_punctuation: true,
        };
        jespersen::write(&sentences, options).map_err(|source| Error::Phonemes {
            text: words.to_owned(),
            writing: true,
            source,
        })
    }
}

/// The reading codes and the language jespersen reads each as. Serbian has one code per
/// script, so each finds its own language pack's abbreviations; jespersen reads both.
const LANGUAGES: &[(&str, Language)] = &[
    ("en-us", Language::AmericanEnglish),
    // As eSpeak NG's British voice: TRAP's vowel in bath, in eSpeak NG's notation.
    ("en", Language::BritishEnglish(BathVowel::ByNotation)),
    ("af", Language::Afrikaans),
    ("ak", Language::Akan),
    ("ca", Language::Catalan),
    ("cs", Language::Czech),
    ("cy", Language::Welsh),
    ("da", Language::Danish),
    ("de", Language::German),
    ("ee", Language::Ewe),
    ("el", Language::Greek),
    ("es", Language::Spanish(SpanishVariety::Castilian)),
    ("et", Language::Estonian),
    ("eu", Language::Basque),
    ("fi", Language::Finnish),
    ("fr", Language::French),
    ("ga", Language::Irish),
    ("ha", Language::Hausa),
    ("hr", Language::Croatian),
    ("hu", Language::Hungarian),
    ("id", Language::Indonesian),
    ("ig", Language::Igbo),
    ("is", Language::Icelandic),
    ("it", Language::Italian),
    ("lg", Language::Luganda),
    ("ln", Language::Lingala),
    ("lt", Language::Lithuanian),
    ("lv", Language::Latvian),
    ("mk", Language::Macedonian),
    ("ms", Language::Malay),
    ("nl", Language::Dutch),
    ("no", Language::Norwegian),
    ("om", Language::Oromo),
    ("pl", Language::Polish),
    ("pt", Language::Portuguese(PortugueseVariety::European)),
    ("rn", Language::Kirundi),
    ("ro", Language::Romanian),
    ("ru", Language::Russian),
    ("rw", Language::Kinyarwanda),
    ("sk", Language::Slovak),
    ("sl", Language::Slovenian),
    ("sn", Language::Shona),
    ("so", Language::Somali),
    ("sq", Language::Albanian),
    ("sr", Language::Serbian),
    ("sr-latn", Language::Serbian),
    ("sv", Language::Swedish),
    ("sw", Language::Swahili),
    ("tr", Language::Turkish),
    ("yo", Language::Yoruba),
];

#[cfg(test)]
#[path = "../../tests/unit/phonemes/jespersen.rs"]
mod tests;
