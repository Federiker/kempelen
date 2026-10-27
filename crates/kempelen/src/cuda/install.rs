use super::{
    ARCHIVES, Archive, CudaError, Format, MARKER, RUNTIME_ID,
    download::{DownloadOptions, download},
    download_bytes,
};
use std::{
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

/// A step of the installation that failed on `path`.
fn file(action: &'static str, path: &Path) -> impl FnOnce(io::Error) -> CudaError {
    let path = path.to_path_buf();
    move |error| CudaError::File {
        action,
        path,
        message: error.to_string(),
    }
}

/// Download, verify, and unpack the runtime into `dir`, replacing any partial install.
///
/// `progress` receives `(downloaded, total)` bytes across every archive. Dropping the
/// future cancels the download; the next call starts again from a clean folder.
///
/// # Errors
///
/// [`CudaError::Unsupported`] off 64-bit Windows; otherwise the archive that could not be
/// downloaded ([`CudaError::Download`], [`CudaError::Size`]) or unpacked ([`CudaError::Unpack`]),
/// or the file that could not be written ([`CudaError::File`]).
pub async fn install(
    dir: &Path,
    progress: impl Fn(u64, u64) + Send + Sync + 'static,
) -> Result<(), CudaError> {
    if !super::available() {
        return Err(CudaError::Unsupported);
    }
    let parent = dir.parent().ok_or_else(|| CudaError::File {
        action: "use",
        path: dir.to_path_buf(),
        message: "Invalid CUDA runtime folder".to_owned(),
    })?;
    let staging = parent.join(format!("{RUNTIME_ID}.partial"));
    let _ = fs::remove_dir_all(&staging);
    let downloads = staging.join("downloads");
    fs::create_dir_all(&downloads).map_err(file("create", &downloads))?;

    let total = download_bytes();
    let progress = Arc::new(progress);
    let mut finished = 0;
    for archive in ARCHIVES {
        let report = progress.clone();
        let options = DownloadOptions {
            sha256: Some(archive.sha256.to_owned()),
            re_download: true,
            timeout_secs: 3600,
            progress: Some(Arc::new(move |done, _| {
                report((finished + done).min(total), total);
            })),
        };
        let path = download(archive.url, &downloads, options)
            .await
            .map_err(|message| CudaError::Download {
                url: archive.url.to_owned(),
                message,
            })?;
        if fs::metadata(&path).map_err(file("read", &path))?.len() != archive.size {
            return Err(CudaError::Size(archive.url.to_owned()));
        }
        extract(&path, archive, &staging)?;
        fs::remove_file(&path).map_err(file("remove", &path))?;
        finished += archive.size;
        progress(finished, total);
    }
    fs::remove_dir_all(&downloads).map_err(file("remove", &downloads))?;
    let marker = staging.join(MARKER);
    fs::write(&marker, RUNTIME_ID).map_err(file("write", &marker))?;
    let _ = fs::remove_dir_all(dir);
    fs::rename(&staging, dir).map_err(file("move the CUDA runtime into", dir))?;
    Ok(())
}

fn extract(archive_path: &Path, archive: &Archive, into: &Path) -> Result<(), CudaError> {
    let unpack = |message: String| CudaError::Unpack {
        path: archive_path.to_path_buf(),
        message,
    };
    let file = BufReader::new(File::open(archive_path).map_err(|error| unpack(error.to_string()))?);
    match archive.format {
        Format::Wheel => extract_wheel(file, archive.files, into),
        Format::TarLzma2 => extract_tar(
            lzma_rust2::Lzma2Reader::new(file, 1 << 26, None),
            archive.files,
            into,
        ),
    }
    .map_err(unpack)
}

/// The output file `name` inside `into`, created.
fn create(into: &Path, name: &str) -> Result<(File, PathBuf), String> {
    let path = into.join(name);
    let file = File::create(&path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    Ok((file, path))
}

/// Copy the wanted `nvidia/<package>/bin/<name>` entries of a wheel into `into`.
fn extract_wheel(reader: impl Read + io::Seek, wanted: &[&str], into: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(reader).map_err(|error| error.to_string())?;
    let mut found = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_owned();
        let Some(file) = name.strip_prefix("nvidia/").and_then(|rest| {
            let (_, file) = rest.split_once("/bin/")?;
            wanted.iter().find(|wanted| **wanted == file)
        }) else {
            continue;
        };
        let (mut out, path) = create(into, file)?;
        io::copy(&mut entry, &mut out)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        found.push(*file);
    }
    missing(wanted, &found)
}

/// Copy the wanted top-level regular files of a ustar stream into `into`.
fn extract_tar(mut reader: impl Read, wanted: &[&str], into: &Path) -> Result<(), String> {
    let mut found = Vec::new();
    let mut header = [0u8; 512];
    loop {
        if reader.read_exact(&mut header).is_err() || header.iter().all(|byte| *byte == 0) {
            break;
        }
        let size = octal(&header[124..136])?;
        let name = field(&header[0..100]);
        let prefix = if &header[257..262] == b"ustar" {
            field(&header[345..500])
        } else {
            String::new()
        };
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let file_name = path.rsplit('/').next().unwrap_or_default();
        let regular = matches!(header[156], b'0' | 0);
        let mut body = (&mut reader).take(size);
        match wanted.iter().find(|wanted| **wanted == file_name) {
            Some(file) if regular => {
                let (mut out, written) = create(into, file)?;
                let copied = io::copy(&mut body, &mut out)
                    .map_err(|error| format!("cannot write {}: {error}", written.display()))?;
                if copied != size {
                    return Err(format!("Truncated archive entry {path}"));
                }
                found.push(*file);
            }
            _ => {
                io::copy(&mut body, &mut io::sink()).map_err(|error| error.to_string())?;
            }
        }
        let padding = (512 - size % 512) % 512;
        io::copy(&mut (&mut reader).take(padding), &mut io::sink())
            .map_err(|error| error.to_string())?;
    }
    missing(wanted, &found)
}

fn field(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn octal(bytes: &[u8]) -> Result<u64, String> {
    let text = field(bytes);
    let text = text.trim();
    if text.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(text, 8).map_err(|_| "Invalid archive entry size".to_owned())
}

fn missing(wanted: &[&str], found: &[&str]) -> Result<(), String> {
    let absent: Vec<_> = wanted.iter().filter(|file| !found.contains(file)).collect();
    if !absent.is_empty() {
        return Err(format!("Archive is missing {absent:?}"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/cuda/install.rs"]
mod tests;
