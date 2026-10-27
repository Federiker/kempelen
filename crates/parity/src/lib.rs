//! The references recorded from manuzio's speech crate, and the checks that hold kempelen (or
//! manuzio, after it switches) to them. See docs/parity.md.

mod digest;
mod phonemes;
mod synthesis;

pub use digest::{Digest, samples_sha256, wav_samples};
pub use phonemes::{PhonemeEntry, PhonemeReferences, check_phonemes};
pub use synthesis::{
    Loudness, Machine, Outcome, Output, Produced, Source, SynthesisCase, SynthesisReference, SynthesisReferences,
    check_synthesis,
};

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
