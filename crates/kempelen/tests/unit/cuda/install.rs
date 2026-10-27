use super::*;
use crate::cuda::PROVIDER;
use std::io::Write;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kempelen-cuda-install-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn wheels_yield_only_their_bin_libraries() {
    let dir = temp_dir("wheel");
    let mut bytes = io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut bytes);
        let options = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("nvidia/cudnn/bin/cudnn64_9.dll", "dll"),
            ("nvidia/cudnn/include/cudnn.h", "header"),
            ("nvidia_cudnn_cu12.dist-info/RECORD", "record"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    bytes.set_position(0);
    extract_wheel(&mut bytes, &["cudnn64_9.dll"], &dir).unwrap();
    assert_eq!(fs::read(dir.join("cudnn64_9.dll")).unwrap(), b"dll");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    bytes.set_position(0);
    assert!(extract_wheel(&mut bytes, &["cudnn_ops64_9.dll"], &dir).is_err());
    fs::remove_dir_all(dir).unwrap();
}

fn tar_entry(out: &mut Vec<u8>, name: &str, body: &[u8], kind: u8) {
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[124..135].copy_from_slice(format!("{:011o}", body.len()).as_bytes());
    header[156] = kind;
    header[257..262].copy_from_slice(b"ustar");
    out.extend_from_slice(&header);
    out.extend_from_slice(body);
    out.resize(out.len().div_ceil(512) * 512, 0);
}

#[test]
fn tar_streams_yield_the_wanted_regular_files() {
    let dir = temp_dir("tar");
    let mut tar = Vec::new();
    tar_entry(&mut tar, "onnxruntime.lib", &[7; 700], b'0');
    tar_entry(
        &mut tar,
        "onnxruntime_providers_cuda.dll",
        b"provider",
        b'0',
    );
    tar_entry(&mut tar, "include/", b"", b'5');
    tar.extend_from_slice(&[0; 1024]);
    extract_tar(tar.as_slice(), &[PROVIDER], &dir).unwrap();
    assert_eq!(fs::read(dir.join(PROVIDER)).unwrap(), b"provider");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    assert!(extract_tar(tar.as_slice(), &["missing.dll"], &dir).is_err());
    fs::remove_dir_all(dir).unwrap();
}
