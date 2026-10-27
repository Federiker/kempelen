#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    clippy::similar_names,
    reason = "tests that measure audio"
)]

use super::*;
use std::{fs::read, path::PathBuf};

fn test_output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("kempelen-{name}-{}.wav", std::process::id()))
}

#[test]
fn writes_mono_24khz_pcm_wav() {
    let path = test_output_path("format");
    let mut writer = WavWriter::create(&path).unwrap();
    assert!(writer.is_empty());
    writer.write(&[-2.0, 0.0]).unwrap();
    assert!(!writer.is_empty());
    writer.write(&[2.0]).unwrap();
    writer.finish().unwrap();

    let bytes = read(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 1);
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        24_000
    );
    // Bytes per second and per sample: 16-bit mono.
    assert_eq!(
        u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
        48_000
    );
    assert_eq!(u16::from_le_bytes([bytes[32], bytes[33]]), 2);
    assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16);
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
    assert_eq!(i16::from_le_bytes([bytes[44], bytes[45]]), -32_767);
    assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), 0);
    assert_eq!(i16::from_le_bytes([bytes[48], bytes[49]]), 32_767);
}

#[test]
fn the_writer_makes_the_bytes_of_the_whole_buffer() {
    let samples: Vec<f32> = (0..10_001)
        .map(|i| (i as f32 * 0.013).sin() * 1.1)
        .collect();
    let path = test_output_path("pieces");
    let mut writer = WavWriter::create(&path).unwrap();
    for piece in samples.chunks(777) {
        writer.write(piece).unwrap();
    }
    writer.silence(300).unwrap();
    assert_eq!(writer.len(), 10_301);
    writer.finish().unwrap();
    let bytes = read(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let mut padded = samples;
    padded.extend(std::iter::repeat_n(0.0, 300));
    assert_eq!(bytes, wav_bytes(&padded).unwrap());
}

#[test]
fn silence_is_rounded_to_the_nearest_sample() {
    assert_eq!(silence_samples(1.5), 36_000);
    assert_eq!(silence_samples(0.12345), 2_963);
    assert_eq!(silence_samples(0.000_02), 0);
    assert_eq!(silence_samples(0.000_03), 1);
    assert_eq!(silence_samples(-1.0), 0);
}
