use super::split_phonemes;

#[test]
fn short_text_is_one_chunk() {
    assert_eq!(split_phonemes("abc def.", 50), vec!["abc def."]);
}

#[test]
fn empty_text_has_no_chunks() {
    assert!(split_phonemes("  ", 10).is_empty());
}

#[test]
fn sentences_are_kept_whole_when_possible() {
    let chunks = split_phonemes("aaaa bbbb. cccc dddd. eeee", 12);
    assert_eq!(chunks, vec!["aaaa bbbb.", "cccc dddd.", "eeee"]);
}

#[test]
fn clauses_are_used_when_no_sentence_end_fits() {
    let chunks = split_phonemes("aaaa bbbb, cccc dddd eeee", 12);
    assert_eq!(chunks, vec!["aaaa bbbb,", "cccc dddd", "eeee"]);
}

#[test]
fn a_long_word_is_cut_as_a_last_resort() {
    let chunks = split_phonemes("abcdefghijkl", 5);
    assert_eq!(chunks, vec!["abcde", "fghij", "kl"]);
}

#[test]
fn no_chunk_exceeds_the_limit_and_no_text_is_lost() {
    let text = "one two, three. four five six! seven eight nine ten eleven twelve";
    let chunks = split_phonemes(text, 17);
    assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 17));
    let joined: String = chunks.join(" ");
    assert_eq!(joined.replace(' ', ""), text.replace(' ', ""));
}
