use super::normalize_text;

#[test]
fn typographic_apostrophes_become_ascii() {
    assert_eq!(normalize_text("it\u{2019}s"), "it's");
}

#[test]
fn guillemets_and_cjk_quotes_become_curly_quotes() {
    assert_eq!(
        normalize_text("\u{00AB}Ciao\u{00BB}"),
        "\u{201C}Ciao\u{201D}"
    );
    assert_eq!(normalize_text("\u{300C}hi\u{300D}"), "\u{201C}hi\u{201D}");
}

#[test]
fn dashes_and_dot_runs_are_collapsed() {
    assert_eq!(
        normalize_text("a \u{2013} b -- c"),
        "a \u{2014} b \u{2014} c"
    );
    assert_eq!(
        normalize_text("wait... what. . ."),
        "wait\u{2026} what. . ."
    );
    assert_eq!(normalize_text("well-known"), "well-known");
}

#[test]
fn fullwidth_punctuation_becomes_ascii() {
    assert_eq!(
        normalize_text("\u{4F60}\u{597D}\u{FF0C}\u{4E16}\u{754C}\u{3002}\u{FF01}\u{FF1F}"),
        "\u{4F60}\u{597D},\u{4E16}\u{754C}.!?"
    );
}

#[test]
fn whitespace_variants_and_invisible_characters_are_cleaned() {
    assert_eq!(normalize_text("a\u{00A0}b\nc\u{00AD}d"), "a b cd");
}
