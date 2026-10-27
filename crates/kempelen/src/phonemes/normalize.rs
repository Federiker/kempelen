//! Text normalization applied before phonemization.
//!
//! Kokoro's vocabulary only knows a handful of punctuation marks. Typographic
//! and fullwidth variants are mapped onto them so that quotes, dashes,
//! ellipses and CJK punctuation still shape the prosody instead of vanishing.

/// Map typographic, fullwidth and spacing characters to Kokoro-friendly ones.
pub fn normalize_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\u{2019}' | '\u{2018}' | '\u{02BC}' | '\u{2032}' => out.push('\''),
            '\u{00AB}' | '\u{201E}' | '\u{201F}' | '\u{2039}' | '\u{300C}' | '\u{300E}' => {
                out.push('\u{201C}');
            }
            '\u{00BB}' | '\u{203A}' | '\u{300D}' | '\u{300F}' => out.push('\u{201D}'),
            '\u{2013}' | '\u{2015}' => out.push('\u{2014}'),
            '-' if chars.peek() == Some(&'-') => {
                while chars.peek() == Some(&'-') {
                    chars.next();
                }
                out.push('\u{2014}');
            }
            '.' if chars.peek() == Some(&'.') => {
                while chars.peek() == Some(&'.') {
                    chars.next();
                }
                out.push('\u{2026}');
            }
            '\u{3002}' | '\u{FF0E}' => out.push('.'),
            '\u{FF0C}' | '\u{3001}' => out.push(','),
            '\u{FF01}' => out.push('!'),
            '\u{FF1F}' => out.push('?'),
            '\u{FF1A}' => out.push(':'),
            '\u{FF1B}' => out.push(';'),
            '\u{FF08}' => out.push('('),
            '\u{FF09}' => out.push(')'),
            '\u{00A0}' | '\u{2009}' | '\u{202F}' | '\t' | '\r' | '\n' => out.push(' '),
            '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
#[path = "../../tests/unit/phonemes/normalize.rs"]
mod tests;
