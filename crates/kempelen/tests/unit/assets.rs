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

#[cfg(feature = "speech-download")]
mod stage {
    use super::super::*;
    use crate::download::tests::{MockResponse, MockServer, TestDir};
    use std::sync::Mutex;

    fn served(path: &str, bytes: &[u8], server: &MockServer) -> Asset {
        Asset {
            url: server.url.clone(),
            ..super::asset(path, bytes)
        }
    }

    #[tokio::test]
    async fn only_missing_or_damaged_files_are_downloaded_and_verified() {
        let dir = TestDir::new();
        let server = MockServer::start(vec![
            MockResponse::ok(b"missing".to_vec()),
            MockResponse::ok(b"damaged".to_vec()),
        ]);
        let files = [
            served("voices/kept.bin", b"kept", &server),
            served("voices/missing.bin", b"missing", &server),
            served("models/damaged.onnx", b"damaged", &server),
        ];
        std::fs::create_dir_all(dir.path().join("voices")).unwrap();
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        std::fs::write(dir.path().join("voices/kept.bin"), b"kept").unwrap();
        std::fs::write(dir.path().join("models/damaged.onnx"), b"broken!").unwrap();
        let reports = Mutex::new(Vec::new());
        stage_files(dir.path(), &files, &|done, total| {
            reports.lock().unwrap().push((done, total));
        })
        .await
        .unwrap();
        assert_eq!(server.finish().len(), 2, "the intact file is not fetched");
        for asset in &files {
            verify(&dir.path().join(&asset.path), asset).unwrap();
        }
        let reports = reports.into_inner().unwrap();
        assert_eq!(reports.first(), Some(&(0, 14)));
        assert_eq!(reports.last(), Some(&(14, 14)));
        let leftovers: Vec<_> = std::fs::read_dir(dir.path().join("voices"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().ends_with(".part"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[tokio::test]
    async fn a_file_larger_than_pinned_or_with_another_checksum_is_refused_and_left_out() {
        for body in [b"too long by far".to_vec(), b"evil".to_vec()] {
            let dir = TestDir::new();
            let server = MockServer::start(vec![MockResponse::ok(body)]);
            let files = [served("voices/test.bin", b"good", &server)];
            let error = stage_files(dir.path(), &files, &|_, _| {})
                .await
                .unwrap_err();
            assert!(
                matches!(&error, Error::Download { path, .. } if path == "voices/test.bin"),
                "{error}"
            );
            server.finish();
            let left: Vec<_> = std::fs::read_dir(dir.path().join("voices"))
                .unwrap()
                .collect();
            assert!(left.is_empty(), "nothing is left behind: {left:?}");
        }
    }

    #[test]
    fn the_pinned_manifest_downloads_only_from_allowed_https_hosts() {
        check_sources(&manifest().files).unwrap();
        let good = Asset {
            url: "https://huggingface.co/onnx-community/x.bin".into(),
            ..super::asset("voices/x.bin", b"x")
        };
        check_sources(std::slice::from_ref(&good)).unwrap();
        for (path, url) in [
            ("voices/x.bin", "http://huggingface.co/x.bin"),
            ("voices/x.bin", "https://example.com/x.bin"),
            ("voices/x.bin", "https://user@huggingface.co/x.bin"),
            ("voices/x.bin", "https://huggingface.co.example.com/x.bin"),
            ("../x.bin", "https://huggingface.co/x.bin"),
            ("voices/../x.bin", "https://huggingface.co/x.bin"),
            ("other/x.bin", "https://huggingface.co/x.bin"),
            ("voices/a/x.bin", "https://huggingface.co/x.bin"),
        ] {
            let asset = Asset {
                path: path.into(),
                url: url.into(),
                ..good.clone()
            };
            assert!(check_sources(&[asset]).is_err(), "{path} {url}");
        }
        assert!(
            check_sources(&[good.clone(), good]).is_err(),
            "a path twice"
        );
    }

    #[test]
    fn the_notice_and_the_manifest_are_written_beside_the_files() {
        let dir = TestDir::new();
        write_notices(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("NOTICE.txt")).unwrap(),
            NOTICE
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("manifest.json")).unwrap(),
            MANIFEST
        );
    }
}
