use crate::speech::data_dir;
use kempelen::{Voice, assets::SpeechData, tts::Synthesizer};
use std::path::PathBuf;

/// Every file under `dir`, with its size and modification time.
fn listing(dir: &std::path::Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut files = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                folders.push(entry.path());
            } else {
                files.push((entry.path(), metadata.len(), metadata.modified().unwrap()));
            }
        }
    }
    files.sort();
    files
}

/// Explicitly requested: never skips missing resources, writes only into a fresh folder of its own
/// (`KEMPELEN_FRESH_DATA_DIR`, which must not exist yet), and leaves the staged data untouched.
#[test]
#[ignore = "requires `mise run speech` and KEMPELEN_FRESH_DATA_DIR naming a folder that does not exist"]
fn first_use_synthesizes_every_shipped_voice_from_the_staged_assets() {
    let fresh = PathBuf::from(
        std::env::var_os("KEMPELEN_FRESH_DATA_DIR").expect("Set KEMPELEN_FRESH_DATA_DIR"),
    );
    assert!(
        !fresh.exists(),
        "Use a fresh folder for this first-use test"
    );
    std::fs::create_dir_all(&fresh).unwrap();
    let staged = data_dir();
    let before = listing(&staged);
    let data = SpeechData::new(&staged);
    assert!(data.model().unwrap().ends_with("models/model.onnx"));
    let voices: Vec<_> = kempelen::assets::manifest()
        .files
        .iter()
        .filter_map(|f| {
            f.path
                .strip_prefix("voices/")
                .and_then(|v| v.strip_suffix(".bin"))
        })
        .collect();
    assert_eq!(voices.len(), 45);
    for name in voices {
        let voice: Voice = serde_json::from_value(serde_json::Value::String(name.into())).unwrap();
        let path = data.voice(voice).unwrap();
        assert!(path.ends_with(format!("voices/{name}.bin")));
        let text = match &name[..1] {
            "z" => "你好，世界。",
            "e" => "Hola, mundo.",
            "f" => "Bonjour, le monde.",
            "i" => "Ciao, mondo.",
            "p" => "Olá, mundo.",
            _ => "Hello, world.",
        };
        let out = fresh.join(format!("{name}.wav"));
        let mut synth = Synthesizer::new(&data, voice, 1.0).unwrap();
        synth.synthesize_to_wav(text, &out).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(&bytes[..4], b"RIFF", "{name}");
        assert!(bytes.len() > 24_000, "Insufficient speech from {name}");
        assert!(
            bytes[44..]
                .chunks_exact(2)
                .any(|b| i16::from_le_bytes([b[0], b[1]]) != 0),
            "Silent voice {name}"
        );
        eprintln!("Offline synthesis verified: {name}");
    }
    assert_eq!(
        listing(&staged),
        before,
        "The staged speech data must not change"
    );
    let written: Vec<_> = std::fs::read_dir(&fresh)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(
        written
            .iter()
            .all(|path| path.extension().is_some_and(|ext| ext == "wav")),
        "Only the WAVs are written, no copy or download: {written:?}"
    );
}
