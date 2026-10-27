use std::path::PathBuf;

/// Why the GPU runtime could not be installed, loaded or used. A caller then synthesizes on the
/// CPU ([`crate::tts::Synthesizer::new`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CudaError {
    /// This build runs on another system than 64-bit Windows.
    #[error("GPU speech acceleration is only available on 64-bit Windows")]
    Unsupported,
    /// The runtime folder lacks the marker of this version or one of its libraries.
    #[error("The GPU speech engine is not installed")]
    NotInstalled,
    /// ONNX Runtime found no CUDA device.
    #[error("No CUDA device is available")]
    NoDevice,
    /// The running program's folder cannot be found.
    #[error("Cannot locate the application folder")]
    NoApplicationFolder,
    /// pyke's shared provider library is not next to the program.
    #[error(
        "{} is missing next to the application in {}; reinstall the application",
        super::SHARED_PROVIDER,
        .0.display()
    )]
    SharedProviderMissing(PathBuf),
    /// A library of the runtime would not load.
    #[error("Cannot load {library}: {message}")]
    Library { library: String, message: String },
    /// Registering the runtime failed earlier in this process, so it is not tried again.
    #[error("{0}")]
    Load(String),
    /// An archive could not be downloaded or failed its checksum.
    #[cfg(feature = "cuda-install")]
    #[error("Failed to download {url}: {message}")]
    Download { url: String, message: String },
    /// A downloaded archive has another size than the pinned one.
    #[cfg(feature = "cuda-install")]
    #[error("Unexpected size for {0}")]
    Size(String),
    /// An archive could not be unpacked, or lacks a library.
    #[cfg(feature = "cuda-install")]
    #[error("Failed to unpack {}: {message}", path.display())]
    Unpack { path: PathBuf, message: String },
    /// A file or folder of the installation could not be created, written, moved or removed.
    #[cfg(feature = "cuda-install")]
    #[error("Cannot {action} {}: {message}", path.display())]
    File {
        action: &'static str,
        path: PathBuf,
        message: String,
    },
}
