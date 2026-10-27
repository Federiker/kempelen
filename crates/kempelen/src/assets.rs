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

#[cfg(test)]
#[path = "../tests/unit/assets.rs"]
mod tests;
