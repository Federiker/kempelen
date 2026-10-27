use super::*;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kempelen-cuda-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn archives_are_pinned_https_downloads_covering_every_preloaded_library() {
    let mut files: Vec<_> = ARCHIVES
        .iter()
        .flat_map(|a| a.files.iter().copied())
        .collect();
    for archive in ARCHIVES {
        assert!(archive.url.starts_with("https://"), "{}", archive.url);
        assert_eq!(archive.sha256.len(), 64);
        assert!(archive.sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }
    assert_eq!(download_bytes(), 1_581_825_296);
    files.sort_unstable();
    let mut expected: Vec<_> = PRELOAD_ORDER.iter().copied().chain([PROVIDER]).collect();
    expected.sort_unstable();
    assert_eq!(files, expected);
}

#[test]
fn the_runtime_is_installed_only_with_its_marker_and_every_library() {
    let dir = temp_dir("installed");
    for file in ARCHIVES.iter().flat_map(|a| a.files) {
        fs::write(dir.join(file), b"dll").unwrap();
    }
    assert!(!is_installed(&dir));
    fs::write(dir.join(MARKER), "older-runtime").unwrap();
    assert!(!is_installed(&dir));
    fs::write(dir.join(MARKER), RUNTIME_ID).unwrap();
    assert!(is_installed(&dir));
    fs::remove_file(dir.join("cudnn64_9.dll")).unwrap();
    assert!(!is_installed(&dir));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn only_64_bit_windows_can_use_the_runtime() {
    // Whether to use it is the application's choice; kempelen reads no environment variable.
    assert_eq!(available(), cfg!(all(windows, target_arch = "x86_64")));
}

#[test]
fn the_runtime_lives_in_its_own_versioned_folder() {
    let dir = runtime_dir(Path::new("data"));
    assert_eq!(dir, Path::new("data").join("cuda").join(RUNTIME_ID));
}
