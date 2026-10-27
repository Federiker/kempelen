//! jespersen must keep its parity with the upstream eSpeak NG reference (development data, never
//! committed: captured into tmp/golden, docs/jespersen.md), the reference must cover the corpus, and
//! the scorer must behave. The tests that need the reference skip themselves without it.

#[path = "../golden/support.rs"]
mod support;

use support::{Entry, MANDARIN, score};

/// The least share of the reference's words jespersen must match, per language: about two points
/// under what it matched on 24 October 2026 (docs/jespersen.md), so that a newer jespersen that
/// breaks a language fails here. Raise a floor when jespersen comes closer.
const JESPERSEN_WORD_FLOORS: [(&str, f64); 32] = [
    ("af", 64.0),
    ("ca", 53.0),
    ("cs", 60.0),
    ("cy", 61.0),
    ("da", 32.0),
    ("de", 60.0),
    ("en", 73.0),
    ("en-us", 74.0),
    ("es", 65.0),
    ("et", 47.0),
    ("eu", 58.0),
    ("fi", 49.0),
    ("fr", 48.0),
    ("ga", 34.0),
    ("hr", 64.0),
    ("hu", 58.0),
    ("id", 56.0),
    ("is", 55.0),
    ("it", 57.0),
    ("lt", 51.0),
    ("lv", 58.0),
    ("nl", 55.0),
    ("no", 48.0),
    ("pl", 64.0),
    ("pt", 49.0),
    ("ro", 52.0),
    ("ru", 33.0),
    ("sk", 57.0),
    ("sl", 58.0),
    ("sq", 57.0),
    ("sv", 55.0),
    ("tr", 32.0),
];

#[test]
fn jespersen_keeps_its_parity_with_the_espeak_reference() {
    let Some(reference) = support::upstream_reference() else {
        eprintln!("skipped: no eSpeak NG reference in tmp/golden (docs/jespersen.md)");
        return;
    };
    let corpus = support::load_corpus().unwrap();
    let current = support::capture_jespersen(&corpus).unwrap();
    let report = support::report(&reference, &current);
    assert_eq!(
        current.keys().map(String::as_str).collect::<Vec<_>>(),
        JESPERSEN_WORD_FLOORS
            .iter()
            .map(|(code, _)| *code)
            .collect::<Vec<_>>(),
        "every corpus language but Mandarin needs a floor"
    );
    assert!(!current.contains_key(MANDARIN));
    for (code, floor) in JESPERSEN_WORD_FLOORS {
        let parity = score(&reference[code], &current[code]);
        assert!(
            parity.word_percent() >= floor,
            "{code} fell under {floor}% of words:\n{report}"
        );
        assert!(
            current[code]
                .iter()
                .all(|entry| !entry.kokoro.starts_with("<error")),
            "{code}: jespersen failed on a line"
        );
    }
}

fn entry(kokoro: &str) -> Entry {
    Entry {
        text: String::new(),
        raw: String::new(),
        kokoro: kokoro.into(),
    }
}

#[test]
fn the_reference_covers_every_corpus_language_without_errors() {
    let Some(reference) = support::upstream_reference() else {
        eprintln!("skipped: no eSpeak NG reference in tmp/golden (docs/jespersen.md)");
        return;
    };
    let corpus = support::load_corpus().unwrap();
    assert_eq!(corpus.languages.len(), 33);
    for (code, lines) in &corpus.languages {
        assert!(lines.len() >= 5, "{code} needs native lines");
        let entries = &reference[code];
        assert_eq!(
            entries.len(),
            lines.len() + corpus.universal.len(),
            "{code}"
        );
        assert!(
            entries
                .iter()
                .all(|e| !e.kokoro.starts_with("<error") && !e.raw.starts_with("<error")),
            "{code}: reference contains an engine error"
        );
    }
}

#[test]
fn scoring_counts_words_even_when_one_is_missing() {
    let parity = score(&[entry("a b c d")], &[entry("a c d")]);
    assert_eq!(
        (parity.exact_lines, parity.words, parity.matched_words),
        (0, 4, 3)
    );
    assert_eq!(
        score(&[entry("x y")], &[entry("x y")]).line_percent(),
        100.0
    );
    assert_eq!(score(&[], &[]).word_percent(), 100.0);
}
