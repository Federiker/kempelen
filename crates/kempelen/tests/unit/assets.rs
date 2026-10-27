use super::*;

fn asset(path: &str, bytes: &[u8]) -> Asset {
    Asset {
        path: path.into(),
        url: String::new(),
        size: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn problem(result: Result<PathBuf>) -> AssetProblem {
    match result {
        Err(Error::Asset { problem, .. }) => problem,
        other => panic!("not an asset error: {other:?}"),
    }
}

#[test]
fn resolves_read_only_bundled_assets_before_legacy_cache_and_rejects_corruption() {
    let root = std::env::temp_dir().join(format!("kempelen-speech-assets-{}", std::process::id()));
    let bundle = root.join("bundle");
    let cache = root.join("cache");
    std::fs::create_dir_all(bundle.join("voices")).unwrap();
    std::fs::create_dir_all(cache.join("voices")).unwrap();
    let path = bundle.join("voices/test.bin");
    let cached = cache.join("voices/test.bin");
    let asset = asset("voices/test.bin", b"test");
    let dirs = [bundle.clone(), cache.clone()];
    std::fs::write(&path, b"test").unwrap();
    std::fs::write(&cached, b"test").unwrap();
    let original_permissions = std::fs::metadata(&path).unwrap().permissions();
    let mut read_only = original_permissions.clone();
    read_only.set_readonly(true);
    std::fs::set_permissions(&path, read_only).unwrap();
    assert_eq!(resolve(&asset, &dirs).unwrap(), path);
    std::fs::set_permissions(&path, original_permissions).unwrap();
    std::fs::write(&path, b"evil").unwrap();
    assert_eq!(problem(resolve(&asset, &dirs)), AssetProblem::Checksum);
    std::fs::write(&path, b"").unwrap();
    assert_eq!(problem(resolve(&asset, &dirs)), AssetProblem::Incomplete);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(resolve(&asset, &dirs).unwrap(), cached);
    std::fs::remove_file(&cached).unwrap();
    let missing = resolve(&asset, &dirs).unwrap_err();
    assert!(
        missing.to_string().contains("no download is attempted"),
        "{missing}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_names_outside_the_manifest_including_paths() {
    let data = SpeechData::new(std::env::temp_dir());
    for name in [
        "../model",
        "models/other.onnx",
        "",
        "voices/../models/model.onnx",
    ] {
        assert_eq!(problem(data.asset(name)), AssetProblem::Unknown, "{name}");
    }
}

#[test]
fn the_manifest_pins_the_model_and_every_voice() {
    let manifest = manifest();
    assert_eq!(manifest.model_license, "Apache-2.0");
    let paths: Vec<&str> = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert!(paths.contains(&"models/model.onnx"));
    for voice in Voice::ALL {
        assert!(
            paths.contains(&format!("voices/{voice}.bin").as_str()),
            "{voice}"
        );
    }
    for file in &manifest.files {
        assert!(file.url.starts_with("https://"), "{}", file.path);
        assert!(file.size > 0 && file.sha256.len() == 64, "{}", file.path);
    }
}

#[test]
fn several_folders_are_searched_in_order() {
    let data = SpeechData::with_dirs(["first", "second"]);
    assert_eq!(
        data.dirs(),
        [PathBuf::from("first"), PathBuf::from("second")]
    );
}
