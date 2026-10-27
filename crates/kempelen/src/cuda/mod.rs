//! Optional NVIDIA CUDA acceleration for Kokoro synthesis (Windows x64).
//!
//! kempelen links pyke's CUDA-enabled static ONNX Runtime 1.24.2 core on 64-bit Windows, which
//! runs on the CPU everywhere. The CUDA execution provider and the NVIDIA libraries it needs
//! (about 1.6 GB to download, 2.3 GB installed) are not bundled: an application installs them
//! once into [`runtime_dir`] (`install`, behind the `cuda-install` feature, downloads the pinned
//! archives), after which [`crate::tts::Synthesizer::with_cuda`] registers them as an ONNX Runtime
//! plugin. The core also needs pyke's `onnxruntime_providers_shared.dll` next to the executable;
//! the application ships it.
//!
//! Every archive is pinned by URL, size, and SHA-256. Those libraries keep NVIDIA's and pyke's
//! terms; kempelen never ships them.

use std::{
    fs,
    path::{Path, PathBuf},
};

mod error;
#[cfg(feature = "cuda-install")]
mod install;

pub use error::CudaError;
#[cfg(feature = "cuda-install")]
pub use install::install;

/// Version of the installed runtime layout; bump whenever [`ARCHIVES`] changes.
pub const RUNTIME_ID: &str = "ort-1.24.2-cu12-1";
/// Human-readable download size for progress messages.
pub const DOWNLOAD_SIZE: &str = "about 1.6 GB";
/// File, beside the application executable, that the static ONNX Runtime core loads
/// before any provider plugin.
pub const SHARED_PROVIDER: &str = "onnxruntime_providers_shared.dll";
const PROVIDER: &str = "onnxruntime_providers_cuda.dll";
const MARKER: &str = ".complete";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    /// A Python wheel (ZIP) whose libraries live under `nvidia/<package>/bin/`.
    Wheel,
    /// pyke's raw LZMA2-compressed tar of a prebuilt ONNX Runtime.
    TarLzma2,
}

#[cfg_attr(
    not(feature = "cuda-install"),
    expect(
        dead_code,
        reason = "only the installer downloads; the files say what is installed"
    )
)]
struct Archive {
    url: &'static str,
    size: u64,
    sha256: &'static str,
    format: Format,
    files: &'static [&'static str],
}

/// Pinned downloads; the provider must come from the same pyke build as the linked core
/// (`ort-sys` 2.0.0-rc.12, feature set `cu12`).
const ARCHIVES: &[Archive] = &[
    Archive {
        url: "https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.2/x86_64-pc-windows-msvc+cu12.tar.lzma2",
        size: 81_934_635,
        sha256: "8a54165e2dfc85e9f6afbdaf154e7c1c74582e6269a2d0ec93b11e1459309555",
        format: Format::TarLzma2,
        files: &[PROVIDER],
    },
    Archive {
        url: "https://files.pythonhosted.org/packages/59/df/e7c3a360be4f7b93cee39271b792669baeb3846c58a4df6dfcf187a7ffab/nvidia_cuda_runtime_cu12-12.9.79-py3-none-win_amd64.whl",
        size: 3_591_604,
        sha256: "8e018af8fa02363876860388bd10ccb89eb9ab8fb0aa749aaf58430a9f7c4891",
        format: Format::Wheel,
        files: &["cudart64_12.dll"],
    },
    Archive {
        url: "https://files.pythonhosted.org/packages/20/e2/fc9a0e985249d873150276d5afb02e39a66817fedbf1a385724393e505ed/nvidia_cublas_cu12-12.9.2.10-py3-none-win_amd64.whl",
        size: 553_162_896,
        sha256: "623f43027d40d44ceadf0043f002bd25cf353e8f13ce90b9a87057019f560661",
        format: Format::Wheel,
        files: &["cublasLt64_12.dll", "cublas64_12.dll"],
    },
    Archive {
        url: "https://files.pythonhosted.org/packages/20/ee/29955203338515b940bd4f60ffdbc073428f25ef9bfbce44c9a066aedc5c/nvidia_cufft_cu12-11.4.1.4-py3-none-win_amd64.whl",
        size: 200_067_309,
        sha256: "8e5bfaac795e93f80611f807d42844e8e27e340e0cde270dcb6c65386d795b80",
        format: Format::Wheel,
        files: &["cufft64_11.dll"],
    },
    Archive {
        url: "https://files.pythonhosted.org/packages/aa/38/f856579877f7c1c5066e61182e7de7bc27bf35a78c8d1b0fa592e6985bc4/nvidia_cudnn_cu12-9.27.0.42-py3-none-win_amd64.whl",
        size: 743_068_852,
        sha256: "06e9b0026f3bad97d2b58666330fabec04fe1672f776661ecb0ce0029c27f142",
        format: Format::Wheel,
        files: &[
            "cudnn64_9.dll",
            "cudnn_graph64_9.dll",
            "cudnn_ops64_9.dll",
            "cudnn_heuristic64_9.dll",
            "cudnn_adv64_9.dll",
            "cudnn_cnn64_9.dll",
            "cudnn_engines_precompiled64_9.dll",
            "cudnn_engines_runtime_compiled64_9.dll",
            "cudnn_engines_tensor_ir64_9.dll",
            "cudnn_ext64_9.dll",
        ],
    },
];

/// NVIDIA libraries in load order. cuDNN opens its sub-libraries by bare name, which
/// Windows only resolves from the default search path or from modules already loaded,
/// so all of them are loaded before the provider.
const PRELOAD_ORDER: &[&str] = &[
    "cudart64_12.dll",
    "cublasLt64_12.dll",
    "cublas64_12.dll",
    "cufft64_11.dll",
    "cudnn64_9.dll",
    "cudnn_graph64_9.dll",
    "cudnn_ops64_9.dll",
    "cudnn_heuristic64_9.dll",
    "cudnn_adv64_9.dll",
    "cudnn_cnn64_9.dll",
    "cudnn_engines_precompiled64_9.dll",
    "cudnn_engines_runtime_compiled64_9.dll",
    "cudnn_engines_tensor_ir64_9.dll",
    "cudnn_ext64_9.dll",
];

/// Total bytes downloaded by `install` (feature `cuda-install`).
pub fn download_bytes() -> u64 {
    ARCHIVES.iter().map(|archive| archive.size).sum()
}

/// Whether this build can use the CUDA runtime: 64-bit Windows only. Whether to use it is the
/// caller's choice (an application may let its users turn it off).
pub fn available() -> bool {
    cfg!(all(windows, target_arch = "x86_64"))
}

/// Folder holding this version of the CUDA runtime inside `data_dir`, a writable folder of the
/// caller's (say, its per-user speech data folder): `<data_dir>/cuda/<RUNTIME_ID>`.
pub fn runtime_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("cuda").join(RUNTIME_ID)
}

/// Whether `dir` holds a complete installation of this runtime version.
pub fn is_installed(dir: &Path) -> bool {
    fs::read_to_string(dir.join(MARKER)).is_ok_and(|id| id.trim() == RUNTIME_ID)
        && ARCHIVES
            .iter()
            .flat_map(|archive| archive.files)
            .all(|file| dir.join(file).is_file())
}

/// Register the installed CUDA provider with ONNX Runtime once per process.
#[cfg(all(windows, target_arch = "x86_64"))]
pub(crate) fn register(dir: &Path) -> Result<(), CudaError> {
    use std::sync::OnceLock;
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| register_once(dir).map_err(|error| error.to_string()))
        .clone()
        .map_err(CudaError::Load)
}

#[cfg(all(windows, target_arch = "x86_64"))]
fn register_once(dir: &Path) -> Result<(), CudaError> {
    if !is_installed(dir) {
        return Err(CudaError::NotInstalled);
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .ok_or(CudaError::NoApplicationFolder)?;
    if !exe_dir.join(SHARED_PROVIDER).is_file() {
        return Err(CudaError::SharedProviderMissing(exe_dir));
    }
    for library in PRELOAD_ORDER {
        ort::util::preload_dylib(dir.join(library)).map_err(|error| CudaError::Library {
            library: (*library).to_owned(),
            message: error.to_string(),
        })?;
    }
    let library = ort::environment::Environment::current()
        .and_then(|environment| environment.register_ep_library("CUDA", dir.join(PROVIDER)))
        .map_err(|error| CudaError::Library {
            library: PROVIDER.to_owned(),
            message: error.to_string(),
        })?;
    // Dropping the handle would unregister the provider; keep it for the process.
    std::mem::forget(library);
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/cuda/tests.rs"]
mod tests;
