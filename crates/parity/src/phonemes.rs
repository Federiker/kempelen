use crate::synthesis::Source;
use serde::{Deserialize, Serialize};

/// One line and how it was pronounced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhonemeEntry {
    /// The reading code (`en-us`, `de`, `cmn`…).
    pub code: String,
    pub text: String,
    /// The engine's output for the whole normalised line, before punctuation handling.
    pub raw: String,
    /// What narration feeds Kokoro (`phonemize_with`).
    pub kokoro: String,
}

/// The phonemes of the golden corpora and of every shipped abbreviation, as manuzio read them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonemeReferences {
    pub source: Source,
    pub entries: Vec<PhonemeEntry>,
}

impl PhonemeReferences {
    /// The committed references (`references/phonemes.json`).
    pub fn committed() -> Self {
        #[expect(
            clippy::expect_used,
            reason = "the committed references are checked by this crate's tests"
        )]
        serde_json::from_str(include_str!("../references/phonemes.json"))
            .expect("valid phoneme references")
    }
}

/// Read every reference line with `read(code, text) -> (raw, kokoro)`, and describe each that differs.
///
/// An engine error is part of what is compared: `read` returns it as `<error>`, as the references do.
pub fn check_phonemes(
    references: &PhonemeReferences,
    mut read: impl FnMut(&str, &str) -> (String, String),
) -> Vec<String> {
    let mut differences = Vec::new();
    for entry in &references.entries {
        let (raw, kokoro) = read(&entry.code, &entry.text);
        if raw != entry.raw || kokoro != entry.kokoro {
            differences.push(format!(
                "[{}] {:?}\n  raw    want {:?}\n         got  {raw:?}\n  kokoro want {:?}\n         got  {kokoro:?}",
                entry.code, entry.text, entry.raw, entry.kokoro
            ));
        }
    }
    differences
}
