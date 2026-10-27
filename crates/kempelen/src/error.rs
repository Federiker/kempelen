use crate::Language;
use std::path::PathBuf;

/// Everything that can go wrong in kempelen (docs/errors.md).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The speed is not a finite number greater than zero.
    #[error("TTS speed must be a finite value greater than zero, not {0}")]
    InvalidSpeed(f32),
    /// Narration cannot pronounce this book language yet.
    #[error("Narration is not available for {0:?} books yet")]
    UnsupportedLanguage(Language),
    /// No phonemizer reads this reading code.
    #[error("jespersen does not read the language {0:?}")]
    UnsupportedReading(String),
    /// The text holds nothing that can be spoken.
    #[error("text produced no Kokoro phonemes")]
    NoSpeech,
    /// A model or voice file is missing, damaged or not in the manifest.
    #[error("Speech asset {path} {problem}")]
    Asset { path: String, problem: AssetProblem },
    /// A voice file is not a list of Kokoro style vectors.
    #[error("Invalid voice file: {0}")]
    InvalidVoice(&'static str),
    /// A voice has no style vector for a sequence this long.
    #[error("Voice has {styles} style vectors, but sequence length is {length}")]
    NoStyle { styles: usize, length: usize },
    /// jespersen could not read or write some words.
    #[error("Cannot {} {text:?}", if *.writing { "write" } else { "read" })]
    Phonemes {
        text: String,
        /// Whether writing the phonemes failed, rather than reading the words.
        writing: bool,
        source: jespersen::Error,
    },
    /// Another [`crate::phonemes::Phonemizer`] failed.
    #[error("The phonemizer failed")]
    Phonemizer(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// ONNX Runtime failed to load the model or to run it.
    #[error(transparent)]
    Model(#[from] ort::Error),
    /// A file could not be created, read or written.
    #[error("Cannot {action} {}", path.display())]
    File {
        /// What was being done: `create`, `read`, `write`.
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    /// A WAV file would be longer than its 32-bit header can say.
    #[error("WAV is too large")]
    WavTooLarge,
    /// A callback (`sink` or `before_chunk`) stopped the synthesis with its own error, which is
    /// kept so the caller can recognise it (`downcast_ref`).
    #[error(transparent)]
    Interrupted(Box<dyn std::error::Error + Send + Sync>),
    /// The GPU runtime could not be installed, loaded or used.
    #[error(transparent)]
    Cuda(#[from] crate::cuda::CudaError),
}

impl Error {
    /// The error a callback returns to stop a synthesis with `error`.
    pub fn interrupted(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Interrupted(error.into())
    }

    pub(crate) fn file(
        action: &'static str,
        path: impl Into<PathBuf>,
    ) -> impl FnOnce(std::io::Error) -> Self {
        let path = path.into();
        move |source| Self::File {
            action,
            path,
            source,
        }
    }
}

/// What is wrong with a speech asset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AssetProblem {
    /// None of the speech data folders holds it. Nothing is ever downloaded at synthesis time.
    #[error("is missing; stage the speech data first (no download is attempted)")]
    Missing,
    /// It cannot be read.
    #[error("cannot be read: {0}")]
    Unreadable(String),
    /// Its size differs from the manifest's.
    #[error("is incomplete; repair or reinstall the speech data")]
    Incomplete,
    /// Its SHA-256 differs from the manifest's.
    #[error("failed its checksum; repair or reinstall the speech data")]
    Checksum,
    /// The manifest does not list it.
    #[error("is not in the speech manifest")]
    Unknown,
}

/// kempelen's result.
pub type Result<T, E = Error> = std::result::Result<T, E>;
