//! The pinned Kokoro model and voices: their manifest, and where a caller keeps them.
//!
//! kempelen never downloads at synthesis time. The caller stages the files listed in [`MANIFEST`]
//! (`scripts/speech.ts` does, with the same checks) and says where they are with [`SpeechData`];
//! every file is checked by byte count and SHA-256 before it is used.

use crate::{AssetProblem, Error, Result, Voice};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// The speech asset manifest (`assets/speech-assets.json`): Kokoro's revision and every file's
/// path, URL, size and SHA-256. Applications staging the same files read it from here.
pub const MANIFEST: &str = include_str!("../assets/speech-assets.json");

/// The notice that goes with the staged files (`assets/speech-NOTICE.txt`).
pub const NOTICE: &str = include_str!("../assets/speech-NOTICE.txt");

/// The parsed [`MANIFEST`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Manifest {
    /// The Hugging Face revision every file is pinned to.
    pub revision: String,
    pub model_license: String,
    pub files: Vec<Asset>,
}

/// One pinned file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct Asset {
    /// Where it goes inside a speech data folder: `models/model.onnx`, `voices/af_heart.bin`.
    pub path: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// The parsed manifest, read once.
pub fn manifest() -> &'static Manifest {
    static MANIFEST_DATA: OnceLock<Manifest> = OnceLock::new();
    MANIFEST_DATA.get_or_init(|| {
        #[expect(
            clippy::expect_used,
            reason = "the embedded manifest is checked by the unit tests"
        )]
        serde_json::from_str(MANIFEST).expect("valid speech asset manifest")
    })
}

/// Where the staged model and voices are: one or more folders, each laid out as the manifest's paths
/// (`models/model.onnx`, `voices/<voice>.bin`). For each file the first folder that has it is used,
/// and must hold it intact; a damaged copy is an error, not a reason to look further.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechData {
    dirs: Vec<PathBuf>,
}

impl SpeechData {
    /// The speech data in `dir`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dirs: vec![dir.into()],
        }
    }

    /// The speech data in the first of `dirs` that has each file (say, the application's bundle, then
    /// an older per-user copy).
    pub fn with_dirs(dirs: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            dirs: dirs.into_iter().map(Into::into).collect(),
        }
    }

    /// The folders searched, in order.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// The verified Kokoro model.
    pub fn model(&self) -> Result<PathBuf> {
        self.asset("models/model.onnx")
    }

    /// The verified file of `voice`.
    pub fn voice(&self, voice: Voice) -> Result<PathBuf> {
        self.asset(&format!("voices/{voice}.bin"))
    }

    /// The verified file at the manifest path `name`.
    pub fn asset(&self, name: &str) -> Result<PathBuf> {
        let asset = manifest()
            .files
            .iter()
            .find(|asset| asset.path == name)
            .ok_or_else(|| Error::Asset {
                path: name.to_owned(),
                problem: AssetProblem::Unknown,
            })?;
        resolve(asset, &self.dirs)
    }
}

fn verify(path: &Path, asset: &Asset) -> Result<()> {
    let problem = |problem| Error::Asset {
        path: asset.path.clone(),
        problem,
    };
    let unreadable = |error: std::io::Error| problem(AssetProblem::Unreadable(error.to_string()));
    let file = File::open(path).map_err(unreadable)?;
    let metadata = file.metadata().map_err(unreadable)?;
    if !metadata.is_file() || metadata.len() != asset.size {
        return Err(problem(AssetProblem::Incomplete));
    }
    let mut reader = BufReader::new(file);
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 65536];
    loop {
        let count = reader.read(&mut buffer).map_err(unreadable)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if format!("{:x}", hash.finalize()) != asset.sha256 {
        return Err(problem(AssetProblem::Checksum));
    }
    Ok(())
}

fn resolve(asset: &Asset, dirs: &[PathBuf]) -> Result<PathBuf> {
    for dir in dirs {
        let path = dir.join(&asset.path);
        if path.exists() {
            verify(&path, asset)?;
            return Ok(path);
        }
    }
    Err(Error::Asset {
        path: asset.path.clone(),
        problem: AssetProblem::Missing,
    })
}

/// Hosts the manifest may download from, as `scripts/speech.ts` allows.
#[cfg(feature = "speech-download")]
const ALLOWED_HOSTS: [&str; 2] = ["huggingface.co", "raw.githubusercontent.com"];

/// Seconds one file may take to download, as in `scripts/speech.ts`.
#[cfg(feature = "speech-download")]
const DOWNLOAD_TIMEOUT_SECS: u64 = 300;

/// Stage the pinned model, voices and notices into `dir` (feature `speech-download`): what
/// `scripts/speech.ts` does, in Rust.
///
/// Every file already there intact is kept; each missing or damaged one is downloaded from the
/// manifest's pinned URL (https, from Hugging Face or GitHub only), refused if larger than pinned,
/// checked by size and SHA-256, and only then moved into place. `NOTICE.txt` and `manifest.json` are
/// written beside them. `progress(done, total)` counts the bytes of the files that are downloaded.
/// About 349 MB on a first run; nothing when everything is staged. Only this call downloads:
/// synthesis never does.
#[cfg(feature = "speech-download")]
pub async fn stage(dir: &Path, progress: impl Fn(u64, u64) + Send + Sync) -> Result<()> {
    let files = &manifest().files;
    check_sources(files)?;
    stage_files(dir, files, &progress).await?;
    write_notices(dir)
}

/// The manifest's checks in `scripts/speech.ts`: safe paths, each once, https from an allowed host,
/// positive sizes and SHA-256 digests.
#[cfg(feature = "speech-download")]
fn check_sources(files: &[Asset]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for asset in files {
        let refuse = |message: &str| Error::Download {
            path: asset.path.clone(),
            message: message.to_owned(),
        };
        let safe_path = asset.path.split_once('/').is_some_and(|(folder, name)| {
            matches!(folder, "models" | "voices" | "notices")
                && !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        }) && !asset.path.contains("..");
        if !safe_path || !seen.insert(asset.path.as_str()) {
            return Err(refuse("invalid speech asset path"));
        }
        let authority = asset
            .url
            .strip_prefix("https://")
            .map(|rest| rest.split(['/', '?', '#']).next().unwrap_or_default());
        let host = authority
            .filter(|authority| !authority.contains('@'))
            .map(|authority| authority.split(':').next().unwrap_or_default());
        if !host.is_some_and(|host| ALLOWED_HOSTS.contains(&host)) {
            return Err(refuse("unsafe speech asset URL"));
        }
        let digest = asset.sha256.len() == 64
            && asset
                .sha256
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
        if asset.size == 0 || !digest {
            return Err(refuse("invalid size or checksum in the manifest"));
        }
    }
    Ok(())
}

/// Download each of `files` that is not in `dir` intact.
#[cfg(feature = "speech-download")]
async fn stage_files(
    dir: &Path,
    files: &[Asset],
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> Result<()> {
    let missing: Vec<&Asset> = files
        .iter()
        .filter(|asset| verify(&dir.join(&asset.path), asset).is_err())
        .collect();
    let total = missing.iter().map(|asset| asset.size).sum();
    let done = std::sync::atomic::AtomicU64::new(0);
    progress(0, total);
    for asset in missing {
        log::info!("Preparing speech asset: {}", asset.path);
        let report = |bytes: u64| {
            let now = done.fetch_add(bytes, std::sync::atomic::Ordering::Relaxed) + bytes;
            progress(now.min(total), total);
        };
        crate::download::download_to(
            &asset.url,
            &dir.join(&asset.path),
            asset.size,
            &asset.sha256,
            DOWNLOAD_TIMEOUT_SECS,
            &report,
        )
        .await
        .map_err(|message| Error::Download {
            path: asset.path.clone(),
            message,
        })?;
    }
    Ok(())
}

/// `NOTICE.txt` and `manifest.json` beside the staged files, as `scripts/speech.ts` writes them.
#[cfg(feature = "speech-download")]
fn write_notices(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(Error::file("create", dir))?;
    for (name, text) in [("NOTICE.txt", NOTICE), ("manifest.json", MANIFEST)] {
        let path = dir.join(name);
        std::fs::write(&path, text).map_err(Error::file("write", &path))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/assets.rs"]
mod tests;
