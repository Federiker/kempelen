use super::*;
use crate::phonemes::Abbreviation;

fn abbreviation(written: &str, spoken: &str) -> Abbreviation {
    Abbreviation {
        written: written.into(),
        spoken: spoken.into(),
        ..Default::default()
    }
}

fn rules(ordinal_dot: bool) -> ReadingRules {
    ReadingRules::new(&Narration {
        ordinal_dot,
        abbreviations: vec![
            abbreviation("Mr.", "Mister"),
            abbreviation("e.g.", "for example"),
            abbreviation("z. B.", "zum Beispiel"),
            Abbreviation {
                before_number: true,
                ..abbreviation("No.", "number")
            },
            Abbreviation {
                can_end_sentence: true,
                ..abbreviation("etc.", "et cetera")
            },
        ],
    })
}

/// Shows kept full stops as `·` for readable expectations.
fn read(rules: &ReadingRules, text: &str) -> String {
    rules.apply(text).replace(KEPT_DOT, "·")
}

#[test]
fn abbreviations_are_spoken_in_full_without_a_sentence_end() {
    let rules = rules(false);
    assert_eq!(read(&rules, "Mr. Smith came."), "Mister Smith came.");
    assert_eq!(
        read(&rules, "Fruit, e.g. apples."),
        "Fruit, for example apples."
    );
    assert_eq!(
        read(&rules, "Obst, z. B. Äpfel."),
        "Obst, zum Beispiel Äpfel."
    );
    // At the start of a sentence, with a capital.
    assert_eq!(read(&rules, "E.g. apples."), "For example apples.");
}

#[test]
fn an_abbreviation_is_a_whole_word() {
    let rules = rules(false);
    assert_eq!(read(&rules, "HMr. Smith"), "HMr. Smith");
    assert_eq!(read(&rules, "Mr.Smith"), "Mister Smith");
}

#[test]
fn a_one_letter_abbreviation_is_not_read_into_an_initial() {
    let rules = ReadingRules::new(&Narration {
        abbreviations: vec![abbreviation("u.", "utca")],
        ..Default::default()
    });
    assert_eq!(read(&rules, "Kossuth u. 3"), "Kossuth utca 3");
    assert_eq!(read(&rules, "U. Kovács"), "U· Kovács");
    let croatian = ReadingRules::new(&Narration {
        abbreviations: vec![abbreviation("g.", "gospodin")],
        ..Default::default()
    });
    assert_eq!(read(&croatian, "e.g. g. Horvat"), "e.g· gospodin Horvat");
}

#[test]
fn a_number_abbreviation_is_spoken_only_before_a_number() {
    let rules = rules(false);
    assert_eq!(read(&rules, "No. 5 is here."), "number 5 is here.");
    assert_eq!(
        read(&rules, "She said No. Then left."),
        "She said No. Then left."
    );
}

#[test]
fn an_abbreviation_that_can_end_a_sentence_keeps_its_stop_before_a_new_one() {
    let rules = rules(false);
    assert_eq!(
        read(&rules, "Cats, dogs, etc. Then"),
        "Cats, dogs, et cetera. Then"
    );
    assert_eq!(read(&rules, "Cats, dogs, etc."), "Cats, dogs, et cetera.");
    assert_eq!(read(&rules, "Cats, etc., and"), "Cats, et cetera, and");
    assert_eq!(
        read(&rules, "Cats, etc. and dogs"),
        "Cats, et cetera and dogs"
    );
}

#[test]
fn a_stop_before_a_lower_case_word_or_after_an_initial_is_no_sentence_end() {
    let rules = rules(false);
    assert_eq!(read(&rules, "approx. ten"), "approx· ten");
    assert_eq!(
        read(&rules, "L. Frank Baum wrote it."),
        "L· Frank Baum wrote it."
    );
    assert_eq!(read(&rules, "(J. Smith)"), "(J· Smith)");
    // A real sentence end stays one.
    assert_eq!(read(&rules, "It rained. Then"), "It rained. Then");
    assert_eq!(read(&rules, "the U.S.A. Then"), "the U.S.A. Then");
}

#[test]
fn ordinals_with_a_full_stop_are_no_sentence_end_where_the_language_writes_them() {
    let german = rules(true);
    assert_eq!(read(&german, "am 3. Mai"), "am 3· Mai");
    assert_eq!(read(&german, "der 100. Geburtstag"), "der 100· Geburtstag");
    // A year still ends a sentence.
    assert_eq!(read(&german, "im Jahr 1999. Dann"), "im Jahr 1999. Dann");
    let english = rules(false);
    assert_eq!(read(&english, "He was 18. Then"), "He was 18. Then");
}

#[test]
fn every_narrated_language_finds_its_own_pack() {
    assert_eq!(pack_language("en-us"), Some(Language::English));
    assert_eq!(pack_language("en"), Some(Language::English));
    assert_eq!(pack_language("cmn"), Some(Language::English));
    assert_eq!(pack_language("de"), Some(Language::German));
    assert_eq!(pack_language("pt"), Some(Language::Portuguese));
    assert_eq!(pack_language("xx"), None);
}

#[test]
fn every_language_ships_its_reading_rules() {
    for language in Language::ALL {
        // Parses, and every entry says something.
        for abbreviation in Narration::shipped(language).abbreviations {
            assert!(!abbreviation.written.trim().is_empty(), "{language:?}");
            assert!(!abbreviation.spoken.trim().is_empty(), "{language:?}");
        }
    }
    assert!(Narration::shipped(Language::German).ordinal_dot);
    assert!(!Narration::shipped(Language::English).ordinal_dot);
    let english = Narration::shipped(Language::English);
    assert!(english.abbreviations.iter().any(|a| a.written == "Dr."));
    assert_eq!(Narration::shipped(Language::Greek), Narration::default());
}

#[test]
fn the_shipped_rules_read_the_text_of_their_code() {
    let mandarin = ReadingRules::for_code("cmn");
    assert_eq!(
        mandarin.apply("Mr. Smith"),
        ReadingRules::for_code("en-us").apply("Mr. Smith")
    );
    assert_eq!(
        ReadingRules::for_code("de")
            .apply("am 3. Mai")
            .replace(KEPT_DOT, "·"),
        "am 3· Mai"
    );
}
