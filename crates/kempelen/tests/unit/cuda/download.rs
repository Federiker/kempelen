use super::{DownloadOptions, download, last_url_part, verify_sha256};
use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let root = env::temp_dir();
        for _ in 0..100 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is before UNIX epoch")
                .as_nanos();
            let path = root.join(format!(
                "kempelen-download-test-{}-{timestamp}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("failed to create test directory: {error}"),
            }
        }
        panic!("could not create a unique download test directory");
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct MockResponse {
    status: &'static str,
    body: Vec<u8>,
}

impl MockResponse {
    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: "200 OK",
            body: body.into(),
        }
    }
}

struct MockServer {
    url: String,
    requests: mpsc::Receiver<String>,
    worker: JoinHandle<()>,
}

impl MockServer {
    fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::channel();
        let worker = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    request.push_str(&line);
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }
                sender.send(request).unwrap();

                let headers = format!(
                    "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response.status,
                    response.body.len()
                );
                let _ = stream.write_all(headers.as_bytes());
                let _ = stream.write_all(&response.body);
                let _ = stream.flush();
            }
        });

        Self {
            url: format!("http://{address}/download.bin"),
            requests,
            worker,
        }
    }

    fn finish(self) -> Vec<String> {
        self.worker.join().unwrap();
        self.requests.try_iter().collect()
    }
}

#[test]
fn sha256_verification_accepts_github_digest_prefix() {
    let temp = TestDir::new();
    let path = temp.path().join("checksum.tmp");
    fs::write(&path, b"abc").unwrap();

    let result = verify_sha256(
        &path,
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );

    assert!(result.unwrap());
}

#[test]
fn sha256_verification_rejects_a_mismatched_digest() {
    let temp = TestDir::new();
    let path = temp.path().join("checksum.tmp");
    fs::write(&path, b"abc").unwrap();

    let result = verify_sha256(&path, &"0".repeat(64));

    assert!(!result.unwrap());
}

#[test]
fn the_file_is_named_after_the_last_decoded_path_segment() {
    for (url, name) in [
        (
            "https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.2/x86_64-pc-windows-msvc+cu12.tar.lzma2",
            "x86_64-pc-windows-msvc+cu12.tar.lzma2",
        ),
        (
            "https://example.com/a/wheel%20name.whl?download=1#top",
            "wheel name.whl",
        ),
        ("https://example.com/folder/", "folder"),
        ("https://example.com", "download"),
    ] {
        assert_eq!(last_url_part(url).unwrap(), name, "{url}");
    }
    assert!(last_url_part("https://example.com/a%2Fb").is_err());
    assert!(last_url_part("https://example.com/%2E%2E").is_err());
}

#[tokio::test]
async fn download_returns_an_error_for_unsuccessful_http_responses() {
    let temp = TestDir::new();
    let server = MockServer::start(vec![MockResponse {
        status: "503 Service Unavailable",
        body: b"temporarily unavailable".to_vec(),
    }]);
    let result = download(&server.url, temp.path(), DownloadOptions::default()).await;

    let error = result.unwrap_err();
    assert!(error.contains("503 Service Unavailable"), "{error}");
    assert!(!temp.path().join("download.bin").exists());
    assert_eq!(server.finish().len(), 1);
}

#[tokio::test]
async fn download_reuses_cached_file_without_another_request() {
    let temp = TestDir::new();
    let server = MockServer::start(vec![MockResponse::ok(b"cached content".to_vec())]);

    let first = download(&server.url, temp.path(), DownloadOptions::default())
        .await
        .unwrap();
    let second = download(&server.url, temp.path(), DownloadOptions::default())
        .await
        .unwrap();

    assert_eq!(first, second);
    assert_eq!(fs::read(second).unwrap(), b"cached content");
    assert_eq!(server.finish().len(), 1);
}

#[tokio::test]
async fn download_reports_progress_up_to_the_full_length() {
    let temp = TestDir::new();
    let server = MockServer::start(vec![MockResponse::ok(b"progress".to_vec())]);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = seen.clone();

    download(
        &server.url,
        temp.path(),
        DownloadOptions {
            progress: Some(std::sync::Arc::new(move |done, total| {
                sink.lock().unwrap().push((done, total));
            })),
            ..DownloadOptions::default()
        },
    )
    .await
    .unwrap();

    let seen = seen.lock().unwrap();
    assert_eq!(seen.first(), Some(&(0, Some(8))));
    assert_eq!(seen.last(), Some(&(8, Some(8))));
    assert_eq!(server.finish().len(), 1);
}

#[tokio::test]
async fn download_verifies_a_supplied_sha256_checksum() {
    let temp = TestDir::new();
    let server = MockServer::start(vec![MockResponse::ok(b"abc".to_vec())]);

    let path = download(
        &server.url,
        temp.path(),
        DownloadOptions {
            sha256: Some(
                "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                    .to_string(),
            ),
            ..DownloadOptions::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(fs::read(path).unwrap(), b"abc");
    assert_eq!(server.finish().len(), 1);
}

#[tokio::test]
async fn download_removes_a_file_when_its_sha256_checksum_does_not_match() {
    let temp = TestDir::new();
    let server = MockServer::start(vec![MockResponse::ok(b"abc".to_vec())]);

    let error = download(
        &server.url,
        temp.path(),
        DownloadOptions {
            sha256: Some("0".repeat(64)),
            ..DownloadOptions::default()
        },
    )
    .await
    .unwrap_err();

    assert!(error.contains("failed SHA-256 verification"), "{error}");
    assert!(!temp.path().join("download.bin").exists());
    assert_eq!(server.finish().len(), 1);
}

#[tokio::test]
async fn a_malformed_checksum_is_refused_before_any_request() {
    let temp = TestDir::new();
    let error = download(
        "http://127.0.0.1:9/never.bin",
        temp.path(),
        DownloadOptions {
            sha256: Some("not hex".to_owned()),
            ..DownloadOptions::default()
        },
    )
    .await
    .unwrap_err();
    assert!(error.contains("64 hexadecimal"), "{error}");
}
