#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests that measure audio"
)]

use super::*;

/// Deterministic "speech": bursts of a mixed tone separated by pauses of near-silence.
fn speech(amplitude: f32, seconds: usize) -> Vec<f32> {
    let mut samples = Vec::new();
    for second in 0..seconds {
        for i in 0..SAMPLE_RATE {
            let t = i as f32 / SAMPLE_RATE as f32;
            let voiced = i < SAMPLE_RATE * 3 / 4;
            let tone = (t * 220.0 * std::f32::consts::TAU).sin()
                + 0.5 * (t * 660.0 * std::f32::consts::TAU).sin();
            // Pauses carry a faint noise floor, about -80 dBFS.
            let noise = (((i + second * 7) * 2_654_435_761usize) % 1000) as f32 / 1000.0 - 0.5;
            samples.push(if voiced {
                amplitude * tone
            } else {
                1.0e-4 * noise
            });
        }
    }
    samples
}

fn db(samples: &[f32]) -> f32 {
    speech_rms_dbfs(samples).expect("enough speech")
}

#[test]
fn quiet_and_loud_voices_end_up_at_the_same_speech_level() {
    for (target, amplitude) in [LoudnessTarget::AUDIOBOOK, LoudnessTarget::RETAIL]
        .into_iter()
        .flat_map(|target| [0.02, 0.1, 0.25].map(|amplitude| (target, amplitude)))
    {
        let mut samples = speech(amplitude, 4);
        let gain = normalize_loudness(&mut samples, target).expect("measurable");
        let level = db(&samples);
        assert!(
            (level - target.speech_rms_dbfs).abs() < 0.2,
            "amplitude {amplitude}: {level} dBFS after a gain of {gain} dB"
        );
        assert!(
            samples
                .iter()
                .all(|s| s.abs() <= target.peak_ceiling + 1e-6)
        );
    }
}

#[test]
fn the_measurement_ignores_pauses() {
    let with_pauses = speech(0.2, 4);
    let speech_only: Vec<f32> = with_pauses
        .chunks(SAMPLE_RATE)
        .flat_map(|second| &second[..SAMPLE_RATE * 3 / 4])
        .copied()
        .collect();
    assert!((db(&with_pauses) - db(&speech_only)).abs() < 0.1);
}

#[test]
fn the_gain_is_bounded_and_peaks_never_exceed_the_ceiling() {
    let mut very_quiet = speech(0.01, 4);
    let gain = normalize_loudness(&mut very_quiet, LoudnessTarget::AUDIOBOOK).unwrap();
    assert!((gain - MAX_BOOST_DB).abs() < 1e-3, "{gain}");

    let mut very_loud = speech(3.0, 4);
    let gain = normalize_loudness(&mut very_loud, LoudnessTarget::AUDIOBOOK).unwrap();
    assert!(gain >= -MAX_CUT_DB - 1e-3, "{gain}");
    assert!(very_loud.iter().all(|s| s.abs() <= 0.7079 + 1e-6));
}

#[test]
fn too_little_speech_is_left_alone_apart_from_limiting() {
    let mut blip: Vec<f32> = vec![0.0; SAMPLE_RATE];
    blip[100] = 1.4;
    assert_eq!(speech_rms_dbfs(&blip), None);
    assert_eq!(
        normalize_loudness(&mut blip, LoudnessTarget::AUDIOBOOK),
        None
    );
    assert!(blip[100] <= 0.7079 + 1e-6);
    assert_eq!(normalize_loudness(&mut [], LoudnessTarget::AUDIOBOOK), None);
}

#[test]
fn a_normalised_book_with_pauses_stays_inside_the_audiobook_retail_window() {
    // Overall RMS, pauses included, must land between -23 and -18 dBFS.
    let mut samples = speech(0.1, 6);
    normalize_loudness(&mut samples, LoudnessTarget::AUDIOBOOK);
    let overall =
        10.0 * (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).log10();
    assert!((-23.0..=-18.0).contains(&overall), "{overall}");
}

#[test]
fn the_meter_gives_the_same_level_whatever_the_pieces() {
    // The level as measured before streaming: whole 20 ms blocks of the whole signal.
    let reference = |samples: &[f32]| {
        let (power, blocks) = samples
            .chunks_exact(BLOCK)
            .map(|block| block.iter().map(|s| s * s).sum::<f32>() / BLOCK as f32)
            .filter(|power| *power > SPEECH_FLOOR_POWER)
            .fold((0.0f64, 0usize), |(sum, count), power| {
                (sum + f64::from(power), count + 1)
            });
        (blocks >= MIN_SPEECH_BLOCKS).then(|| 10.0 * ((power / blocks as f64) as f32).log10())
    };
    let samples: Vec<f32> = (0..50_123)
        .map(|i| 0.2 * (i as f32 * 0.05).sin() * ((i / 2_000) % 3) as f32)
        .collect();
    let expected = reference(&samples);
    assert!(expected.is_some());
    assert_eq!(speech_rms_dbfs(&samples), expected);
    for piece in [1, 13, 479, 480, 481, 4_000] {
        let mut meter = SpeechMeter::default();
        for part in samples.chunks(piece) {
            meter.push(part);
        }
        assert_eq!(meter.speech_rms_dbfs(), expected, "pieces of {piece}");
    }
}
