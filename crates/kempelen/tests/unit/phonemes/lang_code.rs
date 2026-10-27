use super::*;

#[test]
fn a_voice_reading_its_own_language_keeps_its_accent_table() {
    assert_eq!(reading_code(Voice::BfAlice, Language::English), Some("en"));
    assert_eq!(
        reading_code(Voice::AfHeart, Language::English),
        Some("en-us")
    );
    assert_eq!(
        reading_code(Voice::PfDora, Language::Portuguese),
        Some("pt")
    );
}

#[test]
fn other_voices_read_with_the_book_language_rules() {
    assert_eq!(reading_code(Voice::BfAlice, Language::Italian), Some("it"));
    assert_eq!(reading_code(Voice::IfSara, Language::German), Some("de"));
    assert_eq!(
        reading_code(Voice::IfSara, Language::English),
        Some("en-us")
    );
    assert_eq!(
        reading_code(Voice::ZfXiaobei, Language::Norwegian),
        Some("no")
    );
}

#[test]
fn every_language_jespersen_reads_can_be_narrated() {
    for language in Language::ALL {
        let expected = !matches!(
            language,
            Language::Japanese | Language::Ukrainian | Language::Bulgarian
        );
        assert_eq!(can_narrate(language), expected, "{language:?}");
    }
}

#[test]
fn serbian_has_a_reading_code_per_script() {
    assert_eq!(
        reading_code(Voice::BfAlice, Language::SerbianCyrillic),
        Some("sr")
    );
    assert_eq!(
        reading_code(Voice::BfAlice, Language::SerbianLatin),
        Some("sr-latn")
    );
}

#[test]
fn every_reading_code_has_a_phonemizer() {
    for language in Language::ALL {
        for voice in [Voice::BfAlice, Voice::AfHeart, Voice::IfSara] {
            if let Some(code) = reading_code(voice, language) {
                assert!(
                    crate::phonemes::phonemizer_for(code).is_ok(),
                    "{language:?} with {voice:?}: {code}"
                );
            }
        }
    }
}
