//! Model management for the embedded engine: curated catalog, download with
//! progress, on-disk cache and integrity verification. **OUTSIDE the core** — the
//! domain knows nothing about model files. See
//! `docs/architecture/embedded-engine.md` §Public API Contracts.
//!
//! The catalog and `model_dir` come ready (data validated in the spike). TRACK A2
//! implements `local_models`, `download_model` and `delete_model` — without
//! changing the public signatures or the other files in the crate.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wren_core::PortError;

/// A file that makes up a model (downloaded individually).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    pub name: String,
    pub url: String,
    /// Integrity hash, when known. `None` ⇒ check existence/size only.
    pub sha256: Option<String>,
}

/// An entry in the curated catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    /// ISO 639-1 or "auto".
    pub language: String,
    pub size_bytes: u64,
    pub files: Vec<ModelFile>,
}

/// Aggregate progress of a download (sum of all the model's files).
#[derive(Debug, Clone, Copy)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Curated catalog (static). MVP: only Parakeet V3 int8 — multilingual, CPU,
/// validated in the spike (HuggingFace, ~640 MB). See
/// `docs/decisions/0004-inference-engine-selection.md`.
pub fn catalog() -> Vec<ModelInfo> {
    let base = "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8/resolve/main";
    let file = |name: &str| ModelFile {
        name: name.to_string(),
        url: format!("{base}/{name}"),
        sha256: None,
    };
    vec![ModelInfo {
        id: "parakeet-v3-int8".to_string(),
        label: "Parakeet V3 (multilingual, CPU)".to_string(),
        language: "auto".to_string(),
        size_bytes: 640 * 1024 * 1024,
        files: vec![
            file("config.json"),
            file("vocab.txt"),
            file("nemo128.onnx"),
            file("decoder_joint-model.int8.onnx"),
            file("encoder-model.int8.onnx"),
        ],
    }]
}

/// Directory where a downloaded model lives inside the cache: `<cache_root>/<id>/`.
/// It is the path that [`crate::EmbeddedTranscriber`] passes to the worker.
pub fn model_dir(cache_root: &Path, id: &str) -> PathBuf {
    cache_root.join(id)
}

/// Ids of models already present and intact in the cache.
///
/// A model makes the list when its [`model_dir`] exists **and** all the expected
/// files (by name) are there with size > 0. Absence or a missing file ⇒ it simply
/// does not make the list (not an error).
pub fn local_models(cache_root: &Path) -> Vec<String> {
    catalog()
        .into_iter()
        .filter(|model| {
            let dir = model_dir(cache_root, &model.id);
            dir.is_dir()
                && model
                    .files
                    .iter()
                    .all(|f| present_file_len(&dir.join(&f.name)).is_some())
        })
        .map(|model| model.id)
        .collect()
}

/// Downloads all files of model `id` into the cache, reporting aggregate progress
/// via `progress`. Verifies integrity (size and sha256 when available).
/// Idempotent: skips files that are already intact.
///
/// The reported `total` is the model's aggregate — we use the catalog's
/// `size_bytes` (already the sum of all files) as a stable download total.
pub fn download_model(
    cache_root: &Path,
    id: &str,
    progress: &dyn Fn(DownloadProgress),
) -> Result<(), PortError> {
    let info = catalog()
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| PortError::Other(format!("unknown model in the catalog: {id}")))?;

    let dir = model_dir(cache_root, id);
    fs::create_dir_all(&dir)
        .map_err(|e| PortError::Storage(format!("could not create {}: {e}", dir.display())))?;

    download_files(&dir, &info.files, info.size_bytes, progress)
}

/// Removes model `id`'s directory from the cache. **Idempotent**: absence is not an error.
pub fn delete_model(cache_root: &Path, id: &str) -> Result<(), PortError> {
    let dir = model_dir(cache_root, id);
    match fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(PortError::Storage(format!(
            "could not remove {}: {e}",
            dir.display()
        ))),
    }
}

// ── Internals (private) ──────────────────────────────────────────────────────

/// Size of an existing, non-empty file; `None` if absent, empty or not a file.
fn present_file_len(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(|m| m.is_file() && m.len() > 0)
        .map(|m| m.len())
}

/// Lowercase hex of the sha256 of a file's content (streaming, without loading it all).
fn sha256_of_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_lower(&hasher.finalize()))
}

/// Bytes → lowercase hex (avoids depending on a hex crate).
fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// `true` when the file on disk already satisfies the expectation: it exists, is
/// non-empty and, if there is a `sha256` in the catalog, the hash matches. Without
/// a sha ⇒ existence/size is enough.
fn already_complete(dest: &Path, expected_sha: Option<&str>) -> bool {
    if present_file_len(dest).is_none() {
        return false;
    }
    match expected_sha {
        None => true,
        Some(want) => sha256_of_file(dest)
            .map(|got| got.eq_ignore_ascii_case(want))
            .unwrap_or(false),
    }
}

/// Downloads each file with **streaming reqwest blocking** (chunks, never 640 MB
/// in RAM), reporting `progress` with the accumulated `downloaded` over `total`.
///
/// - Idempotent: an already-intact file is skipped (but still counted in progress).
/// - Downloads to `<name>.part` and renames on completion — never leaves a half file.
/// - Verifies `sha256` (when present) after downloading; a mismatch ⇒ deletes and `Other`.
///
/// Testable helper: the tests call it with `ModelFile`s pointing at a local server.
fn download_files(
    dir: &Path,
    files: &[ModelFile],
    total: u64,
    progress: &dyn Fn(DownloadProgress),
) -> Result<(), PortError> {
    let client = reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| PortError::Network(format!("HTTP client: {e}")))?;

    let mut downloaded: u64 = 0;

    for file in files {
        let dest = dir.join(&file.name);

        // Idempotency: already intact ⇒ skip, but count it in progress.
        if already_complete(&dest, file.sha256.as_deref()) {
            downloaded = downloaded.saturating_add(present_file_len(&dest).unwrap_or(0));
            progress(DownloadProgress { downloaded, total });
            continue;
        }

        let mut resp = client
            .get(&file.url)
            .send()
            .map_err(|e| PortError::Network(format!("GET {}: {e}", file.url)))?;

        if !resp.status().is_success() {
            return Err(PortError::Network(format!(
                "HTTP {} while downloading {}",
                resp.status().as_u16(),
                file.name
            )));
        }

        let part = dir.join(format!("{}.part", file.name));
        let mut out = File::create(&part)
            .map_err(|e| PortError::Storage(format!("create {}: {e}", part.display())))?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];

        loop {
            let n = resp
                .read(&mut buf)
                .map_err(|e| PortError::Network(format!("reading {}: {e}", file.name)))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| PortError::Storage(format!("write {}: {e}", part.display())))?;
            if file.sha256.is_some() {
                hasher.update(&buf[..n]);
            }
            downloaded = downloaded.saturating_add(n as u64);
            progress(DownloadProgress { downloaded, total });
        }

        out.flush()
            .and_then(|()| out.sync_all())
            .map_err(|e| PortError::Storage(format!("finalize {}: {e}", part.display())))?;
        drop(out);

        // Integrity check (when the catalog carries the hash).
        if let Some(want) = &file.sha256 {
            let got = hex_lower(&hasher.finalize());
            if !got.eq_ignore_ascii_case(want) {
                let _ = fs::remove_file(&part);
                return Err(PortError::Other(format!(
                    "sha256 mismatch in {}: expected {want}, got {got}",
                    file.name
                )));
            }
        }

        // Publish the file only when intact: atomic rename over the `.part`.
        fs::rename(&part, &dest).map_err(|e| {
            let _ = fs::remove_file(&part);
            PortError::Storage(format!("rename {} → {}: {e}", part.display(), dest.display()))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    // ── Test utilities ───────────────────────────────────────────────────────

    /// A unique temporary directory (removed on `Drop`), without depending on external crates.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let n = SEQ.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir()
                .join(format!("wren-embedded-{tag}-{}-{nanos}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        hex_lower(&h.finalize())
    }

    /// A minimal local HTTP server. Serves, by path (`/name`), the registered
    /// bytes; a missing path ⇒ 404. With `status_override`, it always responds with
    /// that status (empty body). Counts how many requests it served.
    fn serve(
        files: Vec<(String, Vec<u8>)>,
        status_override: Option<u16>,
    ) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let count_srv = count.clone();

        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let mut stream = match conn {
                    Ok(s) => s,
                    Err(_) => break,
                };
                // Read the request headers (GET, no body).
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    if raw.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                count_srv.fetch_add(1, Ordering::Relaxed);

                // Extract the path from the request line "GET /path HTTP/1.1".
                let first = String::from_utf8_lossy(&raw);
                let path = first.lines().next().and_then(|l| l.split_whitespace().nth(1));

                let (header, body): (String, Vec<u8>) = match status_override {
                    Some(st) => (
                        format!(
                            "HTTP/1.1 {st} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        ),
                        Vec::new(),
                    ),
                    None => match path.and_then(|p| {
                        files.iter().find(|(name, _)| name == p).map(|(_, b)| b.clone())
                    }) {
                        Some(bytes) => (
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\
                                 Connection: close\r\n\r\n",
                                bytes.len()
                            ),
                            bytes,
                        ),
                        None => (
                            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\
                             Connection: close\r\n\r\n"
                                .to_string(),
                            Vec::new(),
                        ),
                    },
                };
                use std::io::Write as _;
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
            }
        });

        (format!("http://{addr}"), count)
    }

    fn model_file(base: &str, name: &str, sha: Option<String>) -> ModelFile {
        ModelFile {
            name: name.to_string(),
            url: format!("{base}/{name}"),
            sha256: sha,
        }
    }

    // ── catalog ──────────────────────────────────────────────────────────────

    #[test]
    fn well_formed_catalog() {
        let cat = catalog();
        assert_eq!(cat.len(), 1);
        let m = &cat[0];
        assert_eq!(m.id, "parakeet-v3-int8");
        assert_eq!(m.files.len(), 5);
        assert!(m.size_bytes > 0);
        assert!(m.files.iter().all(|f| f.url.starts_with("https://")));
    }

    // ── local_models ─────────────────────────────────────────────────────────

    fn write_fake(dir: &Path, name: &str, bytes: &[u8]) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), bytes).unwrap();
    }

    #[test]
    fn local_models_empty_cache_lists_empty() {
        let tmp = TempDir::new("empty");
        assert!(local_models(tmp.path()).is_empty());
    }

    #[test]
    fn local_models_complete_cache_lists_the_id() {
        let tmp = TempDir::new("complete");
        let info = &catalog()[0];
        let dir = model_dir(tmp.path(), &info.id);
        for f in &info.files {
            write_fake(&dir, &f.name, b"content");
        }
        assert_eq!(local_models(tmp.path()), vec![info.id.clone()]);
    }

    #[test]
    fn local_models_missing_file_not_listed() {
        let tmp = TempDir::new("incomplete");
        let info = &catalog()[0];
        let dir = model_dir(tmp.path(), &info.id);
        // Write all but the last.
        for f in &info.files[..info.files.len() - 1] {
            write_fake(&dir, &f.name, b"content");
        }
        assert!(local_models(tmp.path()).is_empty());
    }

    #[test]
    fn local_models_empty_file_does_not_count() {
        let tmp = TempDir::new("emptyfile");
        let info = &catalog()[0];
        let dir = model_dir(tmp.path(), &info.id);
        for f in &info.files {
            write_fake(&dir, &f.name, b"x");
        }
        // Zero out one of the files (size 0 ⇒ intact? no).
        fs::write(dir.join(&info.files[0].name), b"").unwrap();
        assert!(local_models(tmp.path()).is_empty());
    }

    // ── delete_model ─────────────────────────────────────────────────────────

    #[test]
    fn delete_model_removes_the_directory() {
        let tmp = TempDir::new("delete");
        let info = &catalog()[0];
        let dir = model_dir(tmp.path(), &info.id);
        write_fake(&dir, &info.files[0].name, b"content");
        assert!(dir.exists());

        delete_model(tmp.path(), &info.id).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn delete_model_absent_is_ok() {
        let tmp = TempDir::new("delete-absent");
        // Never created ⇒ NotFound treated as Ok.
        delete_model(tmp.path(), "parakeet-v3-int8").unwrap();
    }

    // ── download (via testable helper against a local server) ────────────────

    #[test]
    fn download_files_downloads_and_creates_files_with_progress() {
        let tmp = TempDir::new("dl-ok");
        let a = b"content-of-file-a".to_vec();
        let b = b"BBBB".to_vec();
        let (base, _count) = serve(
            vec![("/a.bin".into(), a.clone()), ("/b.bin".into(), b.clone())],
            None,
        );
        let files = vec![
            model_file(&base, "a.bin", None),
            model_file(&base, "b.bin", None),
        ];
        let total = (a.len() + b.len()) as u64;

        let events: Mutex<Vec<DownloadProgress>> = Mutex::new(Vec::new());
        download_files(tmp.path(), &files, total, &|p| {
            events.lock().unwrap().push(p);
        })
        .unwrap();

        // Files created with the right content; no `.part` left over.
        assert_eq!(fs::read(tmp.path().join("a.bin")).unwrap(), a);
        assert_eq!(fs::read(tmp.path().join("b.bin")).unwrap(), b);
        assert!(!tmp.path().join("a.bin.part").exists());

        let events = events.into_inner().unwrap();
        assert!(!events.is_empty(), "progress was never called");
        let last = events.last().unwrap();
        assert_eq!(last.downloaded, total);
        assert_eq!(last.total, total);
    }

    #[test]
    fn download_files_is_idempotent_skips_on_second_call() {
        let tmp = TempDir::new("dl-idem");
        let a = b"AAAAA".to_vec();
        let (base, count) = serve(vec![("/a.bin".into(), a.clone())], None);
        let files = vec![model_file(&base, "a.bin", None)];

        download_files(tmp.path(), &files, a.len() as u64, &|_| {}).unwrap();
        let after_first = count.load(Ordering::Relaxed);
        assert_eq!(after_first, 1, "the first call should download 1 file");

        // Second call: file already intact ⇒ no new request, but progress is still
        // reported (it counts the skip).
        let events: Mutex<Vec<DownloadProgress>> = Mutex::new(Vec::new());
        download_files(tmp.path(), &files, a.len() as u64, &|p| {
            events.lock().unwrap().push(p);
        })
        .unwrap();
        assert_eq!(
            count.load(Ordering::Relaxed),
            after_first,
            "the second call should not make requests"
        );
        let events = events.into_inner().unwrap();
        assert_eq!(events.last().unwrap().downloaded, a.len() as u64);
    }

    #[test]
    fn download_files_http_error_becomes_port_error() {
        let tmp = TempDir::new("dl-500");
        let (base, _count) = serve(vec![], Some(500));
        let files = vec![model_file(&base, "a.bin", None)];

        let err = download_files(tmp.path(), &files, 10, &|_| {}).unwrap_err();
        assert!(matches!(err, PortError::Network(_)), "expected Network, got {err:?}");
        // Nothing was published to the cache.
        assert!(!tmp.path().join("a.bin").exists());
    }

    #[test]
    fn download_files_verifies_correct_sha256() {
        let tmp = TempDir::new("dl-sha-ok");
        let a = b"verified payload".to_vec();
        let sha = sha256_hex(&a);
        let (base, _count) = serve(vec![("/a.bin".into(), a.clone())], None);
        let files = vec![model_file(&base, "a.bin", Some(sha))];

        download_files(tmp.path(), &files, a.len() as u64, &|_| {}).unwrap();
        assert_eq!(fs::read(tmp.path().join("a.bin")).unwrap(), a);
    }

    #[test]
    fn download_files_sha256_mismatch_rejects_and_does_not_publish() {
        let tmp = TempDir::new("dl-sha-bad");
        let a = b"real payload".to_vec();
        let wrong_sha = sha256_hex(b"something else");
        let (base, _count) = serve(vec![("/a.bin".into(), a.clone())], None);
        let files = vec![model_file(&base, "a.bin", Some(wrong_sha))];

        let err = download_files(tmp.path(), &files, a.len() as u64, &|_| {}).unwrap_err();
        assert!(matches!(err, PortError::Other(_)), "expected Other, got {err:?}");
        // Neither the final file nor the `.part` is left behind.
        assert!(!tmp.path().join("a.bin").exists());
        assert!(!tmp.path().join("a.bin.part").exists());
    }

    #[test]
    fn download_model_unknown_id_is_other() {
        let tmp = TempDir::new("dl-unknown");
        let err = download_model(tmp.path(), "does-not-exist", &|_| {}).unwrap_err();
        assert!(matches!(err, PortError::Other(_)), "expected Other, got {err:?}");
    }
}
