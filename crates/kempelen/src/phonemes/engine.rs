use crate::Result;

/// A grapheme-to-phoneme engine for one language.
///
/// Implementations turn a run of words into IPA in the eSpeak NG notation that
/// [`super::phonemize_with`] then fits to Kokoro's vocabulary. Punctuation,
/// spacing and chunking are handled by the caller, so engines only see words
/// (digits and abbreviations included). Keeping this the single seam lets an
/// engine be checked against the golden corpus in `tests/golden`.
pub trait Phonemizer {
    /// Short engine name for reports, e.g. `jespersen`.
    fn engine(&self) -> &'static str;

    /// The reading code of the text it reads (`en-us`, `de`, `cmn`; see `reading_code`), which
    /// selects the language pack's reading rules (see `ReadingRules`).
    fn language(&self) -> &str;

    /// Phonemes for `words`, which contain no punctuation Kokoro understands. An engine other than
    /// kempelen's reports its own failures as [`crate::Error::Phonemizer`].
    fn text_to_phonemes(&self, words: &str) -> Result<String>;
}
