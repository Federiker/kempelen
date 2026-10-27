#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests that measure audio"
)]

use super::*;

fn tone(length: usize, amplitude: f32) -> Vec<f32> {
    (0..length)
        .map(|i| amplitude * (i as f32 * 0.07).sin())
        .collect()
}

#[test]
fn quiet_audio_is_untouched() {
    let original = tone(5_000, 0.5);
    let mut samples = original.clone();
    limit_peaks(&mut samples);
    assert_eq!(samples, original);
}

#[test]
fn over_range_peaks_are_brought_to_the_ceiling_not_clipped_flat() {
    let mut samples = tone(5_000, 1.4);
    let before = samples.clone();
    limit_peaks(&mut samples);
    assert!(samples.iter().all(|s| s.abs() <= CEILING + 1e-6));
    // A clip would flatten every crest at the same value; a limiter keeps the waveform's shape,
    // so neighbouring samples of one crest still differ and the sign never flips.
    assert!(samples.iter().zip(&before).all(|(a, b)| a * b >= 0.0));
    let crest: Vec<f32> = samples
        .windows(3)
        .filter(|w| w[1].abs() > 0.85 && w[1].abs() >= w[0].abs() && w[1].abs() >= w[2].abs())
        .map(|w| w[1].abs())
        .collect();
    assert!(!crest.is_empty());
    // Gain changes smoothly: no step between neighbours beyond the original slope.
    let worst = samples
        .windows(2)
        .zip(before.windows(2))
        .map(|(s, b)| ((s[1] - s[0]) - (b[1] - b[0])).abs())
        .fold(0.0f32, f32::max);
    assert!(worst < 0.1, "gain moved too abruptly: {worst}");
}

#[test]
fn a_single_spike_only_affects_its_neighbourhood() {
    let mut samples = vec![0.1; 1_000];
    samples[500] = 1.6;
    limit_peaks(&mut samples);
    assert!((samples[500].abs() - CEILING).abs() < 1e-5);
    assert_eq!(samples[100], 0.1);
    assert_eq!(samples[900], 0.1);
    assert!(samples[480] < 0.1 && samples[480] > 0.1 * 0.5);
}

#[test]
fn peaks_at_the_edges_and_empty_input_are_handled() {
    limit_peaks(&mut []);
    let mut samples = vec![1.2, 0.0, 0.0, -1.3];
    limit_peaks(&mut samples);
    assert!(samples.iter().all(|s| s.abs() <= CEILING + 1e-6));
    assert!(samples[0] > 0.0 && samples[3] < 0.0);
}

/// The limiter as it was before streaming (a `HashMap` of gains over the whole signal), kept as
/// the reference the streaming one must match sample for sample.
fn reference(samples: &mut [f32], ceiling: f32) {
    let over: Vec<usize> = (0..samples.len())
        .filter(|&index| samples[index].abs() > ceiling)
        .collect();
    if over.is_empty() {
        return;
    }
    let length = samples.len();
    let mut required = std::collections::HashMap::<usize, f32>::new();
    for &index in &over {
        let gain = ceiling / samples[index].abs();
        for at in index.saturating_sub(RADIUS)..=(index + RADIUS).min(length - 1) {
            let entry = required.entry(at).or_insert(1.0);
            *entry = entry.min(gain);
        }
    }
    let gain_at = |at: usize| required.get(&at).copied().unwrap_or(1.0);
    let mut affected: Vec<usize> = required.keys().copied().collect();
    affected.sort_unstable();
    let smoothed: Vec<f32> = affected
        .iter()
        .map(|&at| {
            let from = at.saturating_sub(RADIUS);
            let to = (at + RADIUS).min(length - 1);
            (from..=to).map(gain_at).sum::<f32>() / (to - from + 1) as f32
        })
        .collect();
    for (&at, gain) in affected.iter().zip(smoothed) {
        samples[at] = (samples[at] * gain).clamp(-ceiling, ceiling);
    }
}

/// A small deterministic generator, so the cases are the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn unit(&mut self) -> f32 {
        (self.next() % 1_000_000) as f32 / 1_000_000.0
    }
}

/// Speech-like noise with peaks: sparse, clustered and at both ends.
fn noisy(random: &mut Lcg, length: usize) -> Vec<f32> {
    let mut samples: Vec<f32> = (0..length)
        .map(|i| 0.6 * (i as f32 * 0.031).sin() + 0.3 * (random.unit() - 0.5))
        .collect();
    for _ in 0..length / 300 {
        let at = random.next() as usize % length;
        samples[at] = (0.9 + random.unit() * 0.6) * if random.unit() > 0.5 { 1.0 } else { -1.0 };
    }
    if length > 2 {
        samples[0] = 1.3;
        samples[length - 1] = -1.25;
    }
    samples
}

#[test]
fn the_streaming_limiter_hands_out_a_batch_as_soon_as_it_has_the_context_after_it() {
    // What lets a long chapter stream to the encoder instead of waiting for its end.
    let mut limiter = PeakLimiter::new(CEILING);
    let mut limited = Vec::new();
    limiter.push(&vec![0.1; CONTEXT + BATCH - 1], &mut limited);
    assert!(
        limited.is_empty(),
        "{} samples out too early",
        limited.len()
    );
    limiter.push(&[0.1], &mut limited);
    assert_eq!(limited.len(), BATCH);
    limiter.push(&[0.1; 10], &mut limited);
    assert_eq!(limited.len(), BATCH);
    limiter.finish(&mut limited);
    assert_eq!(limited.len(), CONTEXT + BATCH + 10);
}

#[test]
fn the_streaming_limiter_matches_the_whole_signal_one_whatever_the_pieces() {
    let mut random = Lcg(7);
    for length in [0, 1, 2, 5, 71, 72, 73, 145, 1_000, 8_263, 20_000, 61_234] {
        let original = noisy(&mut random, length);
        let mut expected = original.clone();
        reference(&mut expected, CEILING);
        let mut whole = original.clone();
        limit_peaks(&mut whole);
        assert_eq!(whole, expected, "whole signal of {length} samples");
        for piece in [1, 7, 72, 500, 8_192, 9_000, 0] {
            let mut limiter = PeakLimiter::new(CEILING);
            let mut limited = Vec::new();
            let mut at = 0;
            while at < length {
                // Piece size 0 stands for random sizes.
                let size = if piece == 0 {
                    1 + random.next() as usize % 3_000
                } else {
                    piece
                };
                let end = (at + size).min(length);
                limiter.push(&original[at..end], &mut limited);
                at = end;
            }
            limiter.finish(&mut limited);
            assert_eq!(limited, expected, "{length} samples in pieces of {piece}");
        }
    }
}
