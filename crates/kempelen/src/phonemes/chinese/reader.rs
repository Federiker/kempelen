//! Reading Chinese text the way misaki does; the parent module explains the steps.

use super::numbers;
use jieba_rs::Jieba;
use regex::{Captures, Regex};
use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};

const CHARACTERS: &str = include_str!("../../../mandarin/characters.tsv");
const PHRASES: &str = include_str!("../../../mandarin/phrases.tsv");
const SYLLABLES: &str = include_str!("../../../mandarin/syllables.tsv");
/// OpenCC's character table from Traditional to Simplified (Apache-2.0).
const TRADITIONAL: &str = include_str!("../../../mandarin/TSCharacters.txt");

/// English words inside Chinese text: letters, joined by spaces, apostrophes or hyphens.
#[expect(
    clippy::expect_used,
    reason = "a fixed pattern, exercised by the unit tests"
)]
static LATIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[A-Za-z]+(?:['\u{2019} -][A-Za-z]+)*").expect("valid pattern"));

/// Longest phrase in the pronunciation dictionary, in characters.
const LONGEST_PHRASE: usize = 16;

struct Data {
    jieba: Jieba,
    characters: HashMap<char, &'static str>,
    phrases: HashMap<&'static str, String>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        let table: HashMap<&str, &str> = SYLLABLES
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .collect();
        let characters: HashMap<char, &str> = CHARACTERS
            .lines()
            .filter_map(|line| {
                let (character, syllable) = line.split_once('\t')?;
                Some((character.chars().next()?, *table.get(syllable)?))
            })
            .collect();
        let phrases = PHRASES
            .lines()
            .map(|line| match line.split_once('\t') {
                Some((phrase, syllables)) => (
                    phrase,
                    syllables
                        .split(' ')
                        .filter_map(|syllable| table.get(syllable).copied())
                        .collect::<String>(),
                ),
                None => (
                    line,
                    line.chars()
                        .filter_map(|character| characters.get(&character).copied())
                        .collect(),
                ),
            })
            .collect();
        Data {
            jieba: Jieba::new(),
            characters,
            phrases,
        }
    })
}

/// Traditional characters written as their Simplified forms.
///
/// The pronunciation dictionaries and the word cutter know Simplified text best, so Taiwan and
/// Hong Kong books are read through this first. It maps character by character (OpenCC's first
/// choice, and 著 by the rule below), so a few characters that merge several Traditional ones can read wrongly outside
/// the phrases the dictionary knows. Everything else, Simplified text included, is unchanged.
pub(super) fn to_simplified(text: &str) -> String {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        TRADITIONAL
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let (traditional, simplified) = line.split_once('\t')?;
                let mut keys = traditional.chars();
                let (key, None) = (keys.next()?, keys.next()) else {
                    return None;
                };
                let value = simplified.split(' ').next()?.chars().next()?;
                Some((key, value))
            })
            .collect()
    });
    let characters: Vec<char> = text.chars().collect();
    characters
        .iter()
        .enumerate()
        .map(|(index, character)| {
            if *character == '著' {
                // Traditional writing uses 著 for the particle (數著, read "zhe") that Simplified
                // writes 着; only the words below keep 著 and read "zhù".
                let before = index.checked_sub(1).map(|at| characters[at]);
                let after = characters.get(index + 1).copied();
                let zhu = before
                    .is_some_and(|c| "顯显卓昭編编合原名巨譯译專专遺遗論论拙大".contains(c))
                    || after.is_some_and(|c| "名作稱述者錄書".contains(c));
                return if zhu { '著' } else { '着' };
            }
            map.get(character).copied().unwrap_or(*character)
        })
        .collect()
}

/// Whether `character` is one the Chinese reader pronounces.
fn is_han(character: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&character)
}

/// Kokoro phonemes for Chinese `text`, following misaki's `ZHG2P`, with `latin` pronouncing each
/// run of English words. Other text that is not Han characters (punctuation) is passed through.
pub(super) fn chinese_phonemes_with(text: &str, latin: impl Fn(&str) -> String) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let text = map_punctuation(&numbers::spell_numbers(text));
    let mut out = String::new();
    let mut run = String::new();
    let mut run_is_han = false;
    for character in text.chars() {
        if is_han(character) != run_is_han && !run.is_empty() {
            flush(&mut out, &mut run, run_is_han, &latin);
        }
        run_is_han = is_han(character);
        run.push(character);
    }
    flush(&mut out, &mut run, run_is_han, &latin);
    out
}

fn flush(out: &mut String, run: &mut String, han: bool, latin: &impl Fn(&str) -> String) {
    if han {
        out.push_str(&read_han(run));
    } else {
        out.push_str(&LATIN.replace_all(run, |words: &Captures| latin(&words[0])));
    }
    run.clear();
}

/// Read a run of Han characters: jieba words, joined by spaces.
fn read_han(run: &str) -> String {
    let data = data();
    data.jieba
        .cut(run, true)
        .into_iter()
        .map(|token| read_word(data, token.word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// pypinyin's reading of one word: the longest dictionary phrase at each position, otherwise
/// the character's own reading.
fn read_word(data: &Data, word: &str) -> String {
    let characters: Vec<(usize, char)> = word.char_indices().collect();
    let mut out = String::new();
    let mut at = 0;
    while at < characters.len() {
        let longest = LONGEST_PHRASE.min(characters.len() - at);
        let phrase = (2..=longest).rev().find_map(|length| {
            let start = characters[at].0;
            let end = characters
                .get(at + length)
                .map_or(word.len(), |(offset, _)| *offset);
            data.phrases
                .get(&word[start..end])
                .map(|reading| (length, reading))
        });
        if let Some((length, reading)) = phrase {
            out.push_str(reading);
            at += length;
        } else {
            if let Some(reading) = data.characters.get(&characters[at].1) {
                out.push_str(reading);
            }
            at += 1;
        }
    }
    out
}

/// Misaki's punctuation mapping: Chinese marks become the Latin ones Kokoro knows.
fn map_punctuation(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '、' | '，' => out.push_str(", "),
            '。' | '．' => out.push_str(". "),
            '！' => out.push_str("! "),
            '：' => out.push_str(": "),
            '；' => out.push_str("; "),
            '？' => out.push_str("? "),
            '«' | '《' | '「' | '【' => out.push_str(" “"),
            '»' | '》' | '」' | '】' => out.push_str("” "),
            '（' => out.push_str(" ("),
            '）' => out.push_str(") "),
            other => out.push(other),
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
#[path = "../../../tests/unit/phonemes/chinese.rs"]
mod tests;
