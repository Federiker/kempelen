//! Where the model-backed tests find the staged model and voices.
//!
//! The library never reads the environment; its tests do: `KEMPELEN_DATA_DIR` (mise.toml sets it to
//! `tmp/speech`, filled by `mise run speech`), else `tmp/speech` in the repository.

use kempelen::assets::SpeechData;
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    std::env::var_os("KEMPELEN_DATA_DIR")
        .filter(|dir| !dir.is_empty())
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/speech"),
            PathBuf::from,
        )
}

/// The staged speech data; fails with what to do when the model is not there.
pub fn data() -> SpeechData {
    let data = SpeechData::new(data_dir());
    if let Err(error) = data.model() {
        panic!(
            "The Kokoro model is needed in {} ({error}): run `mise run speech`, or set KEMPELEN_DATA_DIR",
            data_dir().display()
        );
    }
    data
}
