use super::{
    chinese::MandarinPhonemizer,
    engine::Phonemizer,
    jespersen::JespersenPhonemizer,
    lang_code::{get_lang_code, reading_code, unsupported_language},
    normalize::normalize_text,
    reading::{KEPT_DOT, ReadingRules},
};
use crate::{Language, Result, Voice};
use crustytts_tokenizer::KokoroTokenizer;

/// A piece of the input: words for the phonemizer, or one punctuation mark for Kokoro.
#[derive(Debug, PartialEq, Eq)]
enum Piece {
    Words(String),
    Mark(char),
}

/// Convert text to Kokoro-compatible phonemes in the voice's own language.
///
/// The text is normalized first (see [`normalize_text`]). Punctuation known to
/// Kokoro is passed through unchanged, because the model uses it for pauses and
/// intonation; only the words in between go through the phonemizer (jespersen, or the Mandarin
/// front end). Spacing follows the source text.
pub fn get_phonemes(text: &str, voice: Voice) -> Result<String> {
    phonemize(text, get_lang_code(voice))
}

/// Like [`get_phonemes`], but pronounce `text` as `language` (see [`reading_code`]).
///
/// Errors when narration cannot pronounce the language.
pub fn get_phonemes_in(text: &str, voice: Voice, language: Language) -> Result<String> {
    let code = reading_code(voice, language).ok_or_else(|| unsupported_language(language))?;
    phonemize(text, code)
}

/// Phonemize `text` in the reading code `code` (see [`reading_code`]; Mandarin, `cmn`, is read by
/// [`MandarinPhonemizer`], because Kokoro's Chinese voices were trained on misaki's notation).
pub(crate) fn phonemize(text: &str, code: &str) -> Result<String> {
    phonemize_with(&*phonemizer_for(code)?, text)
}

/// The engine that pronounces the reading code `code`: jespersen, or the Mandarin front end.
pub(crate) fn phonemizer_for(code: &str) -> Result<Box<dyn Phonemizer + Send>> {
    if code == "cmn" {
        Ok(Box::new(MandarinPhonemizer::new()))
    } else {
        Ok(Box::new(JespersenPhonemizer::new(code)?))
    }
}

/// Phonemize `text` with an explicit engine, applying the same normalization,
/// abbreviation rules of the engine's language (see [`ReadingRules`]), punctuation handling and
/// Kokoro fitting as narration.
pub fn phonemize_with(engine: &dyn Phonemizer, text: &str) -> Result<String> {
    phonemize_with_rules(engine, &ReadingRules::for_code(engine.language()), text)
}

/// Like [`phonemize_with`], with the caller's reading rules instead of those kempelen ships.
pub fn phonemize_with_rules(
    engine: &dyn Phonemizer,
    rules: &ReadingRules,
    text: &str,
) -> Result<String> {
    let normalized = rules.apply(&normalize_text(text));
    let tokenizer = KokoroTokenizer::new();
    let mut phonemes = String::new();
    let mut pending_space = false;

    for piece in split_pieces(&normalized) {
        let spoken = match piece {
            Piece::Mark(mark) => mark.to_string(),
            Piece::Words(words) => {
                let leading = words.starts_with(char::is_whitespace);
                let trailing = words.ends_with(char::is_whitespace);
                pending_space |= leading;
                let spoken = if words.trim().is_empty() {
                    String::new()
                } else {
                    // An abbreviation is read better with its own full stop than without ("etc.").
                    let words = words.trim().replace(KEPT_DOT, ".");
                    fit_to_kokoro(engine.text_to_phonemes(&words)?.trim(), &tokenizer)
                };
                if spoken.is_empty() {
                    pending_space |= trailing;
                    continue;
                }
                push_piece(&mut phonemes, &spoken, &mut pending_space);
                pending_space = trailing;
                continue;
            }
        };
        push_piece(&mut phonemes, &spoken, &mut pending_space);
    }
    Ok(phonemes)
}

/// Replace symbols Kokoro has no token for with the closest ones it has.
///
/// Kokoro learnt a few languages in eSpeak NG's notation; others use some sounds outside its
/// inventory. Anything still unknown (diacritics) is dropped, as the tokenizer would, and clause
/// breaks become spaces.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the tokenizer is borrowed, as crustytts lends it"
)]
fn fit_to_kokoro(phonemes: &str, tokenizer: &KokoroTokenizer) -> String {
    let mapped: String = phonemes.chars().map(kokoro_symbol).collect();
    let unknown = tokenizer.unknown_chars(&mapped);
    mapped
        .chars()
        .filter(|symbol| *symbol == ' ' || !unknown.contains(symbol))
        .collect()
}

/// The symbol Kokoro reads for `symbol`: itself, or the closest one in its vocabulary.
#[expect(
    clippy::match_same_arms,
    reason = "one arm per sound, each with its reason"
)]
fn kokoro_symbol(symbol: char) -> char {
    match symbol {
        '\n' | '\r' | '\t' => ' ',
        // Welsh "ll" and the retroflex lateral.
        '\u{026C}' | '\u{026D}' => 'l',
        // Polish "ź", to the closest postalveolar.
        '\u{0291}' => '\u{0292}',
        // Scandinavian close central rounded vowel.
        '\u{0289}' => 'u',
        '\u{00E4}' => 'a',
        // The Danish stød, written "?" in eSpeak NG's notation.
        '?' => '\u{0294}',
        // eSpeak NG's notation (so jespersen's) writes Danish ɛ as the Greek ε, which Kokoro drops.
        '\u{03B5}' => '\u{025B}',
        // German short ü, and the rounded central vowel (Swedish, Russian, Dutch, Estonian), to the
        // closest rounded front vowels.
        '\u{028F}' => 'y',
        '\u{0275}' => '\u{00F8}',
        // The dark l (Albanian, Turkish).
        '\u{026B}' => 'l',
        other => other,
    }
}

fn push_piece(phonemes: &mut String, piece: &str, pending_space: &mut bool) {
    if *pending_space && !phonemes.is_empty() {
        phonemes.push(' ');
    }
    *pending_space = false;
    phonemes.push_str(piece);
}

fn split_pieces(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut words = String::new();
    for (index, &character) in chars.iter().enumerate() {
        let next = chars.get(index + 1).copied();
        if is_mark(character, next) {
            if !words.is_empty() {
                pieces.push(Piece::Words(std::mem::take(&mut words)));
            }
            pieces.push(Piece::Mark(character));
        } else {
            words.push(character);
        }
    }
    if !words.is_empty() {
        pieces.push(Piece::Words(words));
    }
    pieces
}

/// Whether `character` is punctuation Kokoro understands.
///
/// Separators inside numbers and abbreviations (`3.5`, `1,000`, `U.S.A`) are
/// left to the phonemizer: they only count when no letter or digit follows.
fn is_mark(character: char, next: Option<char>) -> bool {
    match character {
        ',' | '.' | ':' | ';' => !next.is_some_and(char::is_alphanumeric),
        '!' | '?' | '"' | '(' | ')' | '\u{2014}' | '\u{2026}' | '\u{201C}' | '\u{201D}' => true,
        _ => false,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/phonemes/get_phonemes.rs"]
mod tests;
