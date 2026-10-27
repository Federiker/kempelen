use crate::digest::Digest;
use serde::{Deserialize, Serialize};

/// Where references were recorded from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The program and version recorded, e.g. `manuzio 0ec8e8e (mnz-tts)`.
    pub program: String,
    /// When, as an ISO date.
    pub date: String,
}

/// What the samples depend on besides the code: Kokoro's decoder draws noise in an order that
/// changes with ONNX Runtime's thread count, and ONNX Runtime's kernels differ by platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    pub os: String,
    pub arch: String,
    /// ONNX Runtime's intra-op threads: every logical core.
    pub threads: usize,
}

impl Machine {
    /// This machine.
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            threads: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
        }
    }
}

/// The level a synthesis is brought to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loudness {
    /// The synthesizer's default (-20 dBFS speech, peaks limited at the default ceiling).
    Default,
    /// The voice's own level, peaks still limited.
    None,
    /// The retail target (peaks at -4 dBFS).
    Retail,
}

/// Which way the samples are produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// `synthesize`: the whole buffer.
    Whole,
    /// `synthesize_into`: blocks handed to a sink, with a report before every model chunk.
    Streamed,
    /// `synthesize_to_wav`: the file's bytes.
    Wav,
    /// `synthesize_chunks`: each model output as it came, before levelling.
    Chunks,
}

/// One synthesis, in the library's neutral terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisCase {
    pub name: String,
    /// The voice's serialised name (`af_heart`).
    pub voice: String,
    /// The book language's code (`de`, `zh`, `sr-Latn`), or none to read the voice's own language.
    pub language: Option<String>,
    pub speed: f32,
    pub loudness: Loudness,
    pub output: Output,
    pub text: String,
}

/// What one way of synthesizing produced, before it is digested.
#[derive(Debug, Clone, PartialEq)]
pub enum Produced {
    Whole(Vec<f32>),
    Streamed {
        samples: Vec<f32>,
        /// What `synthesize_into` returned.
        count: usize,
        /// `before_chunk`'s arguments, in order.
        reports: Vec<(usize, usize)>,
    },
    Wav(Vec<u8>),
    Chunks(Vec<Vec<f32>>),
    /// The error's message: the cases that must fail are compared by failing.
    Failed(String),
}

/// The digest of what a case produced, as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    #[serde(flatten)]
    pub digest: Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reports: Option<Vec<(usize, usize)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Vec<Digest>>,
    /// Only that it failed is compared, not the message, which kempelen words its own way.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
}

impl Outcome {
    pub fn of(produced: &Produced) -> Self {
        let plain = |digest| Self {
            digest,
            count: None,
            reports: None,
            chunks: None,
            failed: false,
        };
        match produced {
            Produced::Whole(samples) => plain(Digest::of_samples(samples)),
            Produced::Streamed {
                samples,
                count,
                reports,
            } => Self {
                count: Some(*count),
                reports: Some(reports.clone()),
                ..plain(Digest::of_samples(samples))
            },
            Produced::Wav(bytes) => plain(Digest::of_wav(bytes)),
            Produced::Chunks(chunks) => Self {
                chunks: Some(
                    chunks
                        .iter()
                        .map(|chunk| Digest::of_samples(chunk))
                        .collect(),
                ),
                ..plain(Digest::of_samples(&chunks.concat()))
            },
            Produced::Failed(_) => Self {
                failed: true,
                ..plain(Digest::of_samples(&[]))
            },
        }
    }
}

/// A case and what manuzio produced for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisReference {
    #[serde(flatten)]
    pub case: SynthesisCase,
    pub expected: Outcome,
}

/// The syntheses recorded on one machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisReferences {
    pub source: Source,
    pub machine: Machine,
    pub cases: Vec<SynthesisReference>,
}

impl SynthesisReferences {
    /// The committed references (`references/synthesis.json`).
    pub fn committed() -> Self {
        #[expect(
            clippy::expect_used,
            reason = "the committed references are checked by this crate's tests"
        )]
        serde_json::from_str(include_str!("../references/synthesis.json"))
            .expect("valid synthesis references")
    }
}

/// Run every case with `run`, and describe each whose outcome differs from the reference.
pub fn check_synthesis(
    references: &SynthesisReferences,
    mut run: impl FnMut(&SynthesisCase) -> Produced,
) -> Vec<String> {
    let mut differences = Vec::new();
    for reference in &references.cases {
        let produced = run(&reference.case);
        let outcome = Outcome::of(&produced);
        if outcome != reference.expected {
            let error = match &produced {
                Produced::Failed(message) => format!(" (error: {message})"),
                _ => String::new(),
            };
            differences.push(format!(
                "{}: want {:?}\n  got {outcome:?}{error}",
                reference.case.name, reference.expected
            ));
        }
    }
    differences
}
