//! Streaming HTTP downloads with SHA-256 verification: `download`, the part of manuzio's downloader
//! the CUDA installer uses (`cuda::install`), and `download_to`, the way manuzio's `scripts/speech.ts`
//! stages a speech file (`assets::stage`).
#![cfg_attr(
    not(feature = "cuda-install"),
    allow(dead_code, reason = "only the CUDA installer downloads into a folder")
)]

use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Receives `(downloaded_bytes, total_bytes)` while a response arrives; the total is
/// `None` when the server does not report a length.
pub(crate) type DownloadProgress = Arc<dyn Fn(u64, Option<u64>) + Send + Sync>;

/// Options controlling HTTP requests, caching and checksum verification.
pub(crate) struct DownloadOptions {
    /// Download again even when a cached output already exists.
    pub re_download: bool,
    /// Request timeout, in seconds.
    pub timeout_secs: u64,
    /// Expected SHA-256 digest, optionally prefixed with `sha256:`.
    pub sha256: Option<String>,
    /// Report progress to the caller.
    pub progress: Option<DownloadProgress>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            re_download: false,
            timeout_secs: 1200,
            sha256: None,
            progress: None,
        }
    }
}

/// Download a URL into the folder `to`, named after the URL's final path segment, optionally
/// verifying its checksum. Errors are described in words, for the caller to wrap.
///
/// Cached files are reused unless `options.re_download` is true.
pub(crate) async fn download(
    url: &str,
    to: &Path,
    options: DownloadOptions,
) -> Result<PathBuf, String> {
    fs::create_dir_all(to).map_err(|error| format!("cannot create {}: {error}", to.display()))?;
    if let Some(checksum) = &options.sha256 {
        parse_sha256(checksum)?;
    }
    let out_path = to.join(last_url_part(url)?);

    if out_path.exists() && !options.re_download {
        let checksum_matches = match &options.sha256 {
            Some(checksum) => verify_sha256(&out_path, checksum)?,
            None => true,
        };
        if checksum_matches {
            return Ok(out_path);
        }
        fs::remove_file(&out_path).map_err(|error| error.to_string())?;
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(options.timeout_secs))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("download failed: {error}"))?;

    let length = response.content_length();
    let mut file = File::create(&out_path)
        .map_err(|error| format!("cannot create {}: {error}", out_path.display()))?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    if let Some(report) = &options.progress {
        report(0, length);
    }
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("download failed: {error}"))?;
        file.write_all(&chunk)
            .map_err(|error| format!("cannot write {}: {error}", out_path.display()))?;
        downloaded += chunk.len() as u64;
        if let Some(report) = &options.progress {
            report(downloaded, length);
        }
    }
    drop(file);

    if let Some(checksum) = &options.sha256
        && !verify_sha256(&out_path, checksum)?
    {
        let _ = fs::remove_file(&out_path);
        return Err("downloaded file failed SHA-256 verification".to_owned());
    }
    Ok(out_path)
}

/// Download `url` to `path` as `scripts/speech.ts` stages a speech file: into a temporary
/// `<name>.<unique>.part` beside it, refusing more than `size` bytes, then checking the size and the
/// SHA-256 and only then renaming it into place. The temporary file never stays behind, and a file
/// already at `path` is replaced only by a verified one. `progress` receives each chunk's length.
#[cfg(feature = "speech-download")]
pub(crate) async fn download_to(
    url: &str,
    path: &Path,
    size: u64,
    sha256: &str,
    timeout_secs: u64,
    progress: &(dyn Fn(u64) + Send + Sync),
) -> Result<(), String> {
    let expected = parse_sha256(sha256)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("no file name in {}", path.display()))?;
    let folder = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(folder)
        .map_err(|error| format!("cannot create {}: {error}", folder.display()))?;
    let temporary = folder.join(format!("{name}.{}.part", unique_id()));
    let result = fetch_verified(url, &temporary, size, &expected, timeout_secs, progress).await;
    let result = result.and_then(|()| {
        fs::rename(&temporary, path)
            .map_err(|error| format!("cannot move it to {}: {error}", path.display()))
    });
    let _ = fs::remove_file(&temporary);
    result
}

#[cfg(feature = "speech-download")]
async fn fetch_verified(
    url: &str,
    temporary: &Path,
    size: u64,
    expected: &[u8; 32],
    timeout_secs: u64,
    progress: &(dyn Fn(u64) + Send + Sync),
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("download failed: {error}"))?;
    let write_error =
        |error: std::io::Error| format!("cannot write {}: {error}", temporary.display());
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .map_err(write_error)?;
    let mut hash = Sha256::new();
    let mut received = 0u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("download failed: {error}"))?;
        received += chunk.len() as u64;
        if received > size {
            return Err(format!("larger than its pinned {size} bytes"));
        }
        hash.update(&chunk);
        file.write_all(&chunk).map_err(write_error)?;
        progress(chunk.len() as u64);
    }
    file.flush().map_err(write_error)?;
    if received != size || hash.finalize().as_slice() != expected {
        return Err("checksum mismatch".to_owned());
    }
    Ok(())
}

/// A name part no other download of this process, or of another one, uses at the same time.
#[cfg(feature = "speech-download")]
fn unique_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    format!(
        "{}-{nanos}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// The decoded final non-empty path segment of a URL, as a file name.
fn last_url_part(url: &str) -> Result<String, String> {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let path = path.split_once("://").map_or(path, |(_, rest)| rest);
    let path = path
        .split_once('/')
        .map_or("", |(_, path)| path)
        .trim_end_matches('/');
    let name = path
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("download");
    let name = percent_decode(name)?;
    if matches!(name.as_str(), "." | "..") || name.contains(['/', '\\']) {
        return Err(format!("unsafe file name in {url}"));
    }
    Ok(name)
}

fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        if let Some(byte) = escaped {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| format!("the file name {text:?} is not UTF-8"))
}

fn parse_sha256(checksum: &str) -> Result<[u8; 32], String> {
    let checksum = checksum
        .trim()
        .strip_prefix("sha256:")
        .or_else(|| checksum.trim().strip_prefix("SHA256:"))
        .unwrap_or(checksum.trim());
    if checksum.len() != 64 {
        return Err("SHA-256 checksum must contain 64 hexadecimal characters".to_owned());
    }
    let mut bytes = [0u8; 32];
    for (index, chunk) in checksum.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = std::str::from_utf8(chunk)
            .ok()
            .and_then(|digits| u8::from_str_radix(digits, 16).ok())
            .ok_or("SHA-256 checksum is not valid hexadecimal")?;
    }
    Ok(bytes)
}

fn verify_sha256(path: &Path, expected: &str) -> Result<bool, String> {
    let expected = parse_sha256(expected)?;
    let read_error = |error: std::io::Error| format!("cannot read {}: {error}", path.display());
    let mut file = File::open(path).map_err(read_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let bytes_read = file.read(&mut buffer).map_err(read_error)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(hasher.finalize().as_slice() == expected)
}

#[cfg(test)]
#[path = "../tests/unit/download.rs"]
pub(crate) mod tests;
