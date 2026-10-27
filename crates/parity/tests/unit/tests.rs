use crate::{Digest, Outcome, PhonemeReferences, Produced, SynthesisReferences};

#[test]
fn the_committed_references_parse() {
    let phonemes = PhonemeReferences::committed();
    let synthesis = SynthesisReferences::committed();
    assert!(!phonemes.entries.is_empty());
    assert!(!synthesis.cases.is_empty());
}

#[test]
fn samples_are_digested_by_their_bits() {
    let digest = Digest::of_samples(&[0.0, -0.0]);
    assert_eq!(digest.samples, 2);
    assert_ne!(
        digest,
        Digest::of_samples(&[0.0, 0.0]),
        "-0.0 is other bits"
    );
}

#[test]
fn a_wav_counts_the_samples_after_its_header() {
    let mut wav = vec![0u8; 44];
    wav.extend([1, 0, 2, 0]);
    assert_eq!(Digest::of_wav(&wav).samples, 2);
}

#[test]
fn chunks_are_digested_one_by_one_and_together() {
    let outcome = Outcome::of(&Produced::Chunks(vec![vec![0.5], vec![0.25, 0.125]]));
    assert_eq!(outcome.digest, Digest::of_samples(&[0.5, 0.25, 0.125]));
    assert_eq!(outcome.chunks.unwrap().len(), 2);
}

#[test]
fn a_failure_is_compared_by_failing_only() {
    let one = Outcome::of(&Produced::Failed("one message".into()));
    let other = Outcome::of(&Produced::Failed("another".into()));
    assert!(one.failed);
    assert_eq!(one, other);
}
