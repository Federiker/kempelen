use super::{LONGEST_PHRASE, chinese_phonemes_with, data};
use crate::phonemes::{MandarinPhonemizer, Phonemizer, phonemize_with};
use crustytts_tokenizer::KokoroTokenizer;
use std::{collections::HashMap, fs, path::PathBuf};

/// The reader alone: English words are passed through instead of read.
fn chinese_phonemes(text: &str) -> String {
    chinese_phonemes_with(text, ToString::to_string)
}

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

#[test]
fn the_longest_dictionary_phrase_fits_the_matching_window() {
    let longest = data()
        .phrases
        .keys()
        .map(|phrase| phrase.chars().count())
        .max()
        .expect("phrases are loaded");
    assert!(longest <= LONGEST_PHRASE, "phrases of {longest} characters");
}

#[test]
fn a_sentence_is_read_with_tone_arrows() {
    assert_eq!(chinese_phonemes("你好，世界！"), "ni↓xau↓, ʂɨ↘ʨje↘!");
}

#[test]
fn polyphonic_characters_follow_their_word() {
    // 行 is "háng" in a bank and "xíng" in walking.
    let bank = chinese_phonemes("银行");
    let walk = chinese_phonemes("行走");
    assert!(bank.contains("xa"), "{bank}");
    assert!(walk.starts_with("ɕi"), "{walk}");
}

#[test]
fn every_phoneme_is_one_kokoro_knows() {
    let tokenizer = KokoroTokenizer::new();
    let text = fs::read_to_string(golden("mandarin-corpus.json")).expect("read the corpus");
    let corpus: Vec<String> = serde_json::from_str(&text).expect("corpus is a list of texts");
    // Latin letters are the one thing misaki leaves for the caller; see the next test.
    for sentence in corpus
        .iter()
        .filter(|sentence| !sentence.contains(|c: char| c.is_ascii_alphabetic()))
    {
        let phonemes = chinese_phonemes(sentence);
        let unknown = tokenizer.unknown_chars(&phonemes);
        assert!(unknown.is_empty(), "{sentence}: {unknown:?}");
    }
}

#[test]
fn english_words_are_read_as_english_and_never_reach_the_model_as_letters() {
    let spoken =
        phonemize_with(&MandarinPhonemizer::new(), "我用Python写代码。").expect("Mandarin is read");
    assert!(!spoken.contains('P') && !spoken.contains("yth"), "{spoken}");
    assert!(spoken.contains("pˈaɪθɑːn"), "{spoken}");
    assert!(spoken.ends_with('.'), "{spoken}");
}

#[test]
fn narration_keeps_marks_and_fits_the_vocabulary() {
    let phonemes = phonemize_with(
        &MandarinPhonemizer::new(),
        "我们走吧，好吗？《红楼梦》很长。",
    )
    .expect("Mandarin needs no helper");
    assert!(phonemes.contains(','), "{phonemes}");
    assert!(phonemes.contains('?'), "{phonemes}");
    assert!(KokoroTokenizer::new().unknown_chars(&phonemes).is_empty());
    assert_eq!(MandarinPhonemizer::new().engine(), "mandarin");
}

/// The Rust reader gives exactly what misaki gives on the recorded corpus (see
/// `tests/golden/misaki_reference.py`, which also lists how the corpus is built).
#[test]
fn mandarin_matches_misaki() {
    let corpus: Vec<String> = serde_json::from_str(
        &fs::read_to_string(golden("mandarin-corpus.json")).expect("read the corpus"),
    )
    .expect("corpus is a list of texts");
    let expected: HashMap<String, String> = serde_json::from_str(
        &fs::read_to_string(golden("mandarin-misaki.json")).expect("read misaki's output"),
    )
    .expect("misaki output maps text to phonemes");
    let different: Vec<(&String, String, &String)> = corpus
        .iter()
        .filter_map(|text| {
            let ours = chinese_phonemes(text);
            (ours != expected[text]).then(|| (text, ours, &expected[text]))
        })
        .collect();
    for (text, ours, theirs) in different.iter().take(5) {
        eprintln!("{text}\n  ours   {ours}\n  misaki {theirs}");
    }
    assert!(
        different.is_empty(),
        "{} of {} sentences differ from misaki",
        different.len(),
        corpus.len()
    );
}

#[test]
fn traditional_text_is_read_like_the_same_words_in_simplified_characters() {
    let engine = MandarinPhonemizer::new();
    for (traditional, simplified) in [
        (
            "老燈塔守每天晚上都要爬上樓梯，一邊走一邊數著每一級台階。",
            "老灯塔守每天晚上都要爬上楼梯，一边走一边数着每一级台阶。",
        ),
        (
            "銀行裡的錢不夠，他只好去問問。",
            "银行里的钱不够，他只好去问问。",
        ),
        ("這是一本關於海洋與風的書。", "这是一本关于海洋与风的书。"),
        // 乾 and 幹 both become 干; the dictionary knows which word is which.
        ("乾淨的幹部", "干净的干部"),
    ] {
        assert_eq!(
            engine.text_to_phonemes(traditional).unwrap(),
            engine.text_to_phonemes(simplified).unwrap(),
            "{traditional}"
        );
    }
}

#[test]
fn simplified_text_and_shared_characters_are_not_touched() {
    use super::to_simplified;
    assert_eq!(to_simplified("老灯塔守每天晚上"), "老灯塔守每天晚上");
    assert_eq!(to_simplified("著名的台灣"), "著名的台湾");
    // 著 is the particle "zhe" unless it sits in one of the words that read "zhù".
    assert_eq!(to_simplified("數著，看著，想著"), "数着，看着，想着");
    assert_eq!(to_simplified("著作與顯著"), "著作与显著");
    assert_eq!(to_simplified("显著的成就，名著"), "显著的成就，名著");
    assert_eq!(to_simplified("English 25℃ ，。"), "English 25℃ ，。");
}
