/// Split a phoneme string into chunks of at most `max` characters.
///
/// Kokoro accepts a limited sequence length, so long passages are cut at the
/// strongest boundary available: sentence end, then a clause mark, then a
/// space, and only as a last resort in the middle of a word. Chunks are
/// trimmed and never empty.
pub fn split_phonemes(phonemes: &str, max: usize) -> Vec<String> {
    assert!(max > 0, "max must be positive");
    let mut chunks = Vec::new();
    let mut rest: Vec<char> = phonemes.trim().chars().collect();
    while rest.len() > max {
        let cut = cut_point(&rest[..=max]);
        let tail = rest.split_off(cut);
        push_trimmed(&mut chunks, &rest);
        rest = tail;
        let skip = rest.iter().take_while(|c| c.is_whitespace()).count();
        rest.drain(..skip);
    }
    push_trimmed(&mut chunks, &rest);
    chunks
}

fn push_trimmed(chunks: &mut Vec<String>, chars: &[char]) {
    let chunk: String = chars.iter().collect::<String>().trim().to_string();
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
}

/// Index where to cut a window that is one character longer than the limit.
fn cut_point(window: &[char]) -> usize {
    let limit = window.len() - 1;
    let after_last = |matches: fn(char) -> bool| {
        window
            .iter()
            .rposition(|&c| matches(c))
            .map(|index| index + 1)
            .filter(|&cut| cut > 0 && cut <= limit + 1)
    };
    // A closing mark right at the limit must still fit in the chunk.
    after_last(|c| matches!(c, '.' | '!' | '?' | '\u{2026}'))
        .or_else(|| after_last(|c| matches!(c, ',' | ';' | ':' | '\u{2014}')))
        .or_else(|| after_last(char::is_whitespace))
        .map_or(limit, |cut| cut.min(limit))
}

#[cfg(test)]
#[path = "../../tests/unit/phonemes/split.rs"]
mod tests;
