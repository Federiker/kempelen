use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// The length and SHA-256 of some samples or of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest {
    /// Samples (for a WAV file, those in its data chunk).
    pub samples: usize,
    pub sha256: String,
}

impl Digest {
    /// The digest of `samples`, by their little-endian `f32` bytes, so equal digests are equal bits.
    pub fn of_samples(samples: &[f32]) -> Self {
        Self {
            samples: samples.len(),
            sha256: samples_sha256(samples),
        }
    }

    /// The digest of a 16-bit WAV file's bytes, header included.
    pub fn of_wav(bytes: &[u8]) -> Self {
        Self {
            samples: wav_samples(bytes),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }
}

/// SHA-256 of `samples` as little-endian `f32` bytes.
pub fn samples_sha256(samples: &[f32]) -> String {
    let mut hash = Sha256::new();
    for sample in samples {
        hash.update(sample.to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

/// Samples in a mono 16-bit WAV file with the plain 44-byte header.
pub fn wav_samples(bytes: &[u8]) -> usize {
    bytes.len().saturating_sub(44) / 2
}
