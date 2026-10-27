use super::*;

fn read(code: &str, text: &str) -> String {
    JespersenPhonemizer::new(code)
        .unwrap()
        .text_to_phonemes(text)
        .unwrap()
}

#[test]
fn english_is_read_in_espeak_notation() {
    assert_eq!(read("en-us", "Hello there"), "həlˈoʊ ðˈɛɹ");
    assert_eq!(read("en", "Hello there"), "həlˈəʊ ðˈeə");
}

#[test]
fn numbers_are_read_in_the_language() {
    assert_eq!(read("en-us", "42"), "fˈɔːɹɾi tˈuː");
    let italian = read("it", "42");
    assert!(
        italian.contains("kwaɾ") || italian.contains("kwar"),
        "{italian}"
    );
}

#[test]
fn every_reading_code_is_read() {
    for (code, _) in LANGUAGES {
        let phonemizer = JespersenPhonemizer::new(code).unwrap();
        assert_eq!(phonemizer.language(), *code);
        assert!(
            !phonemizer.text_to_phonemes("1 2 3").unwrap().is_empty(),
            "{code}"
        );
    }
}

#[test]
fn mandarin_and_unknown_codes_are_refused() {
    for code in ["cmn", "ja", "uk", ""] {
        assert!(JespersenPhonemizer::new(code).is_err(), "{code}");
    }
}

#[test]
fn a_latin_word_the_language_cannot_read_is_read_as_english() {
    let russian = read("ru", "Он сказал hello");
    assert!(russian.ends_with("həlˈoʊ"), "{russian}");
}

#[test]
fn unreadable_symbols_are_left_silent() {
    assert_eq!(
        read("en-us", "Hello ★ 😀 there"),
        read("en-us", "Hello there")
    );
    assert_eq!(read("it", "★"), "");
    // Another script inside English text, which no retry can read.
    assert_eq!(read("en-us", "Tokyo 東京"), read("en-us", "Tokyo"));
}

#[test]
fn serbian_is_read_in_either_script() {
    assert_eq!(read("sr", "Добро јутро"), read("sr-latn", "Dobro jutro"));
}

#[test]
fn full_stops_inside_the_words_are_not_written() {
    // The full stops of initials and abbreviations the caller kept inside the words.
    assert!(!read("en-us", "L. Frank Baum").contains('.'));
    assert!(!read("it", "Mr. Smith e U.S.A.").contains('.'));
}

#[test]
fn an_initial_does_not_end_the_sentence_for_kokoro() {
    let phonemes = crate::phonemes::get_phonemes(
        "They met L. Frank Baum. Then they left.",
        crate::Voice::AfHeart,
    )
    .unwrap();
    assert_eq!(phonemes.matches('.').count(), 2, "{phonemes}");
}
