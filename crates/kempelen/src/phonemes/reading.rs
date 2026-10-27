//! Abbreviations, and full stops that do not end a sentence.
//!
//! The splitter in `get_phonemes` gives Kokoro every full stop before a space as a sentence end,
//! with its falling tone and pause. Before that, the language's abbreviations
//! ([`Narration`]) are replaced by what the voice should say, and full stops that
//! cannot end a sentence become [`KEPT_DOT`], so they stay inside the words sent to the phonemizer.

use super::lang_code::reading_code;
use crate::{Language, Voice};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

/// A full stop that is not a sentence end; it becomes a full stop again in the words given to
/// the phonemizer (a private-use character, so it never occurs in a book).
pub(crate) const KEPT_DOT: char = '\u{E000}';

/// How narration reads a language's written conventions: the data [`ReadingRules`] are made from.
///
/// kempelen ships one per language ([`Narration::shipped`]), in the JSON form of this type; a caller may
/// pass its own instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Narration {
    /// The language writes ordinal numbers with a full stop (German "am 3. Mai"), so a short
    /// number followed by a full stop and a word does not end a sentence.
    #[serde(default)]
    pub ordinal_dot: bool,
    /// Abbreviations read in full, so their full stop is not heard as the end of a sentence.
    #[serde(default)]
    pub abbreviations: Vec<Abbreviation>,
}

/// A written abbreviation and what the voice says for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Abbreviation {
    /// As written, with its full stops and spaces ("Dr.", "z. B."); matched as a whole word, and
    /// also with its first letter in capitals at the start of a sentence.
    pub written: String,
    /// What the voice says, in words the language's pronunciation reads correctly ("Doktor").
    pub spoken: String,
    /// Only before a number ("No. 5"); elsewhere the word is read as it is written.
    #[serde(default)]
    pub before_number: bool,
    /// Often closes a sentence ("etc."): before a capitalised word, or at the end of the text,
    /// its full stop stays a sentence end.
    #[serde(default)]
    pub can_end_sentence: bool,
}

impl Narration {
    /// The rules kempelen ships for `language` (`data/reading-rules.json`), empty for a language
    /// without any.
    pub fn shipped(language: Language) -> Self {
        static SHIPPED: OnceLock<HashMap<String, Narration>> = OnceLock::new();
        SHIPPED
            .get_or_init(|| {
                #[expect(
                    clippy::expect_used,
                    reason = "the shipped rules are checked by the unit tests"
                )]
                serde_json::from_str(include_str!("../../data/reading-rules.json"))
                    .expect("valid reading rules")
            })
            .get(language.code())
            .cloned()
            .unwrap_or_default()
    }
}

/// One written form of an abbreviation.
#[derive(Debug)]
struct Pattern {
    written: Vec<char>,
    spoken: String,
    before_number: bool,
    can_end_sentence: bool,
}

/// How one language's text is prepared for reading: its abbreviations and number conventions.
#[derive(Debug, Default)]
pub struct ReadingRules {
    /// Longest written form first, so "z. B." is tried before "B.".
    patterns: Vec<Pattern>,
    ordinal_dot: bool,
}

impl ReadingRules {
    /// Rules from a language's [`Narration`].
    pub fn new(narration: &Narration) -> Self {
        let mut patterns = Vec::new();
        for abbreviation in &narration.abbreviations {
            let written = abbreviation.written.trim();
            let spoken = abbreviation.spoken.trim();
            if written.is_empty() || spoken.is_empty() {
                continue;
            }
            let mut forms = vec![(written.to_string(), spoken.to_string())];
            let capital = (capitalise(written), capitalise(spoken));
            // "u." (utca) in capitals would be an initial: "U. Kovács".
            let single_letter = written.chars().filter(|c| c.is_alphanumeric()).count() == 1;
            if capital.0 != written && !single_letter {
                forms.push(capital);
            }
            for (written, spoken) in forms {
                patterns.push(Pattern {
                    written: written.chars().collect(),
                    spoken,
                    before_number: abbreviation.before_number,
                    can_end_sentence: abbreviation.can_end_sentence,
                });
            }
        }
        patterns.sort_by(|a, b| b.written.len().cmp(&a.written.len()));
        Self {
            patterns,
            ordinal_dot: narration.ordinal_dot,
        }
    }

    /// The shipped rules for text pronounced with the reading code `code`, made once per language.
    /// Mandarin (`cmn`) gets the English rules, for the English words inside Chinese text.
    pub fn for_code(code: &str) -> Arc<Self> {
        static CACHE: OnceLock<Mutex<HashMap<String, Arc<ReadingRules>>>> = OnceLock::new();
        let cache = CACHE.get_or_init(Mutex::default);
        let mut cache = cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache
            .entry(code.to_string())
            .or_insert_with(|| {
                let narration = pack_language(code)
                    .map(Narration::shipped)
                    .unwrap_or_default();
                Arc::new(Self::new(&narration))
            })
            .clone()
    }

    /// `text` with abbreviations spoken in full and every full stop that does not end a sentence
    /// replaced by [`KEPT_DOT`]. Expects normalised text (see `normalize_text`).
    pub fn apply(&self, text: &str) -> String {
        let spoken = self.expand(&text.chars().collect::<Vec<_>>());
        self.keep_inner_stops(&spoken)
    }

    fn expand(&self, text: &[char]) -> Vec<char> {
        let mut out = Vec::with_capacity(text.len());
        let mut index = 0;
        while index < text.len() {
            // Not after a full stop either: "g." (gospodin) is no word inside "e.g.".
            let starts_word =
                index == 0 || !(text[index - 1].is_alphanumeric() || text[index - 1] == '.');
            let found = starts_word
                .then(|| {
                    self.patterns
                        .iter()
                        .find(|pattern| matches_at(text, index, pattern))
                })
                .flatten();
            if let Some(pattern) = found {
                out.extend(pattern.spoken.chars());
                index += pattern.written.len();
                if pattern.can_end_sentence && ends_sentence(text, index) {
                    out.push('.');
                } else if text.get(index).is_some_and(|c| c.is_alphanumeric()) {
                    // "Mr.Smith": the spoken form must not run into the next word.
                    out.push(' ');
                }
            } else {
                out.push(text[index]);
                index += 1;
            }
        }
        out
    }

    fn keep_inner_stops(&self, text: &[char]) -> String {
        let mut out: Vec<char> = text.to_vec();
        for (index, &character) in text.iter().enumerate() {
            if character != '.' || !text.get(index + 1).is_some_and(|c| c.is_whitespace()) {
                continue;
            }
            let next = text[index + 1..].iter().find(|c| !c.is_whitespace());
            let word = word_before(text, index);
            let keep = match next {
                // A full stop before a lower-case word is never a sentence end ("etc. and").
                Some(next) if next.is_lowercase() => true,
                // An initial ("L. Frank Baum"), or an ordinal ("am 3. Mai") where the language
                // writes them with a full stop.
                Some(next) if next.is_uppercase() => {
                    is_initial(&word) || (self.ordinal_dot && is_ordinal(&word))
                }
                Some(next) if next.is_alphanumeric() => self.ordinal_dot && is_ordinal(&word),
                _ => false,
            };
            if keep {
                out[index] = KEPT_DOT;
            }
        }
        out.into_iter().collect()
    }
}

/// The language whose shipped rules apply to the reading code `code`.
fn pack_language(code: &str) -> Option<Language> {
    match code {
        "en-us" | "en" | "cmn" => Some(Language::English),
        _ => Language::ALL
            .into_iter()
            .find(|&language| reading_code(Voice::default(), language) == Some(code)),
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Whether `pattern` is written at `index` as a whole word (and before a number, if it must be).
fn matches_at(text: &[char], index: usize, pattern: &Pattern) -> bool {
    let end = index + pattern.written.len();
    if end > text.len() || text[index..end] != pattern.written[..] {
        return false;
    }
    // A form that ends in a letter must end the word: "No" is not the start of "Nobody".
    let last_is_letter = pattern.written.last().is_some_and(|c| c.is_alphanumeric());
    if last_is_letter && text.get(end).is_some_and(|c| c.is_alphanumeric()) {
        return false;
    }
    if !pattern.before_number {
        return true;
    }
    text[end..]
        .iter()
        .find(|c| !c.is_whitespace())
        .is_some_and(char::is_ascii_digit)
}

/// Whether the text after `index` starts a new sentence: nothing left, or a capitalised word.
fn ends_sentence(text: &[char], index: usize) -> bool {
    match text[index..].iter().find(|c| !c.is_whitespace()) {
        None => true,
        Some(next) => next.is_uppercase() && text.get(index).is_some_and(|c| c.is_whitespace()),
    }
}

/// The run of non-space characters before `index`, without opening quotes and brackets.
fn word_before(text: &[char], index: usize) -> Vec<char> {
    let start = text[..index]
        .iter()
        .rposition(|c| c.is_whitespace())
        .map_or(0, |space| space + 1);
    let word = &text[start..index];
    let skip = word.iter().take_while(|c| !c.is_alphanumeric()).count();
    word[skip..].to_vec()
}

fn is_initial(word: &[char]) -> bool {
    matches!(word, [letter] if letter.is_uppercase())
}

/// One to three digits: a day or a position, not a year ("1999. Then" keeps its sentence end).
fn is_ordinal(word: &[char]) -> bool {
    (1..=3).contains(&word.len()) && word.iter().all(char::is_ascii_digit)
}

#[cfg(test)]
#[path = "../../tests/unit/phonemes/reading.rs"]
mod tests;
