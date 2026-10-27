use super::{JespersenPhonemizer, Phonemizer, Piece, kokoro_symbol, normalize_text, split_pieces};
use crustytts_tokenizer::KokoroTokenizer;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn marks_are_split_from_words() {
    assert_eq!(
        split_pieces("Hi, you!"),
        vec![
            Piece::Words("Hi".into()),
            Piece::Mark(','),
            Piece::Words(" you".into()),
            Piece::Mark('!'),
        ]
    );
}

#[test]
fn separators_inside_numbers_stay_with_the_words() {
    assert_eq!(
        split_pieces("3.5 and 1,000"),
        vec![Piece::Words("3.5 and 1,000".into())]
    );
}

#[test]
fn quotes_and_brackets_are_marks() {
    let pieces = split_pieces("\u{201C}a\u{201D} (b)");
    assert_eq!(
        pieces
            .iter()
            .filter(|p| matches!(p, Piece::Mark(_)))
            .count(),
        4
    );
}

/// A line in each language the golden corpus does not cover yet.
const LINES_OUTSIDE_THE_CORPUS: [(&str, &str); 18] = [
    ("el", "Κάθε πρωί ο γέρος περπατά ως το ποτάμι."),
    ("sw", "Kila asubuhi mzee hutembea hadi mtoni."),
    ("rw", "Buri gitondo umusaza agenda ku mugezi."),
    ("rn", "Buri gitondo umutama aja ku ruzi."),
    ("so", "Subax kasta odayga wuxuu u socdaa webiga."),
    ("om", "Ganama ganama jaarsichi gara lagaatti deema."),
    (
        "lg",
        "Buli nkya omusajja omukadde atambula okugenda ku mugga.",
    ),
    ("ln", "Ntɔngɔ nyonso mobange atambolaka tii na ebale."),
    (
        "sn",
        "Mangwanani ese murume mukuru anofamba achienda kurwizi.",
    ),
    ("mk", "Секое утро старецот оди до реката."),
    ("sr", "Сваког јутра старац шета до реке."),
    ("sr-latn", "Svakog jutra starac šeta do reke."),
    ("ms", "Setiap pagi, lelaki tua itu berjalan ke sungai."),
    ("yo", "Ní àràárọ̀, arúgbó náà máa ń rìn lọ sí odò."),
    ("ig", "Kwa ụtụtụ, agadi nwoke ahụ na-aga n'osimiri."),
    ("ha", "Kowace safiya tsohon mutumin yana tafiya zuwa kogi."),
    ("ee", "Ŋdi sia ŋdi la, ŋutsu tsitsia zɔna yia tɔsisi la to."),
    ("ak", "Anɔpa biara akɔkoraa no nantew kɔ asubɔnten no ho."),
];

/// A letter Kokoro has no token for is dropped with its sound; diacritics and eSpeak NG's markers
/// (`#`, `^`) may be.
#[test]
fn every_letter_jespersen_writes_reaches_kokoro() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/corpus.json");
    let corpus: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let universal = corpus["universal"].as_array().unwrap();
    let mut texts: Vec<(String, String)> = Vec::new();
    for (code, lines) in corpus["languages"].as_object().unwrap() {
        if code == "cmn" {
            continue;
        }
        for line in lines.as_array().unwrap().iter().chain(universal) {
            texts.push((code.clone(), line.as_str().unwrap().to_owned()));
        }
    }
    for (code, line) in LINES_OUTSIDE_THE_CORPUS {
        texts.push((code.to_owned(), line.to_owned()));
    }
    let tokenizer = KokoroTokenizer::new();
    let mut dropped: BTreeMap<String, BTreeSet<char>> = BTreeMap::new();
    for (code, text) in texts {
        let raw = JespersenPhonemizer::new(&code)
            .unwrap()
            .text_to_phonemes(&normalize_text(&text))
            .unwrap();
        let mapped: String = raw.chars().map(kokoro_symbol).collect();
        for symbol in tokenizer.unknown_chars(&mapped) {
            if symbol.is_alphabetic() {
                dropped.entry(code.clone()).or_default().insert(symbol);
            }
        }
    }
    assert!(dropped.is_empty(), "Kokoro would drop {dropped:?}");
}

#[test]
fn italian_long_palatal_l_is_read_as_kokoro_says_it_well() {
    // Long after the stressed vowel, as jespersen writes it there; short ones and other languages
    // keep ʎ.
    assert_eq!(super::kokoro_sounds("mˈɛʎʎo", "it"), "mˈɛlʲːo");
    assert_eq!(super::kokoro_sounds("fˈiʎʎo e ʎɪ", "it"), "fˈilʲːo e ʎɪ");
    assert_eq!(super::kokoro_sounds("zbaʎˈavo", "it"), "zbaʎˈavo");
    assert_eq!(super::kokoro_sounds("ˈmuʎʎeɾ", "es"), "ˈmuʎʎeɾ");
    assert_eq!(super::kokoro_sounds("fˈiʎʎo", "pt"), "fˈiʎʎo");
}

#[test]
fn italian_text_reaches_kokoro_without_a_long_palatal_l() {
    let phonemes = crate::phonemes::get_phonemes(
        "Gli gnocchi sono meglio, e io sbagliavo.",
        crate::Voice::IfSara,
    )
    .unwrap();
    assert!(!phonemes.contains("ʎʎ"), "{phonemes}");
    let unknown = crustytts_tokenizer::KokoroTokenizer::new().unknown_chars(&phonemes);
    assert!(unknown.is_empty(), "{unknown:?} in {phonemes}");
}
