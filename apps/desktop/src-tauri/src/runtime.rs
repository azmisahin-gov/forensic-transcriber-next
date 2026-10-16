//! Runtime management.
//!
//! A non-technical user must never have to find a binary, name a model file or
//! set an environment variable. This module:
//!
//! * discovers an already-present `whisper-cli` and model (env override, app
//!   directory, data directory, `PATH`), and
//! * downloads the official whisper.cpp Windows CPU build and a GGML model on
//!   first use, verifying a pinned SHA-256 before anything is accepted.
//!
//! Nothing is downloaded without an explicit user action. The download verifies
//! the hash of the official release archive and the model file.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use ftn_integrity::hash_file;
use thiserror::Error;

/// The whisper.cpp release used for the in-app engine download.
pub const ENGINE_VERSION: &str = "v1.9.2";
pub const ENGINE_ZIP_URL: &str =
    "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.2/whisper-bin-x64.zip";
/// SHA-256 of `whisper-bin-x64.zip` from the official v1.9.2 release.
pub const ENGINE_ZIP_SHA256: &str =
    "49dcc16de826f20bd53d44f947a1ae49dfa81f86cad67a64d80820cb192d674a";

/// A known GGML model that can be downloaded. URLs point at the official
/// `ggerganov/whisper.cpp` Hugging Face repository.
pub struct ModelSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size_bytes: u64,
}

pub const MODELS: &[ModelSpec] = &[ModelSpec {
    id: "base",
    label: "Base — küçük ve hızlı, çok dilli (Türkçe dahil)",
    file: "ggml-base.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    size_bytes: 147_951_465,
}];

pub fn model_spec(id: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.id == id)
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("ağ hatası: {0}")]
    Http(String),
    #[error("dosya hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("arşiv hatası: {0}")]
    Zip(String),
    #[error("sha256 uyuşmazlığı: beklenen {expected}, gelen {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("işlem iptal edildi")]
    Cancelled,
    #[error("bilinmeyen model: {0}")]
    UnknownModel(String),
    #[error("motor yürütülebilir dosyası bulunamadı")]
    EngineMissing,
}

pub fn models_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models")
}

pub fn runtime_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("runtime")
}

pub fn engine_dir(data_dir: &Path) -> PathBuf {
    runtime_dir(data_dir).join("whisper").join("Release")
}

pub fn engine_binary(data_dir: &Path) -> PathBuf {
    engine_dir(data_dir).join("whisper-cli.exe")
}

/// Search order for `whisper-cli`: explicit override, data dir, paths relative
/// to the executable (dev builds live under `target/<profile>`), then `PATH`.
pub fn find_engine(
    exe_dir: &Path,
    data_dir: &Path,
    env_override: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(p) = env_override {
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    let mut candidates = vec![
        engine_binary(data_dir),
        runtime_dir(data_dir)
            .join("whisper")
            .join("whisper-cli.exe"),
    ];
    for up in ["", "..", "../..", "../../..", "../../../.."] {
        let base = exe_dir.join(up);
        candidates.push(
            base.join("runtime")
                .join("whisper")
                .join("Release")
                .join("whisper-cli.exe"),
        );
        candidates.push(base.join("runtime").join("whisper").join("whisper-cli.exe"));
    }
    for candidate in &candidates {
        if candidate.is_file() {
            return Some(candidate.clone());
        }
    }
    which("whisper-cli")
}

/// Search order for a model: explicit setting, data dir `models/`, paths
/// relative to the executable, then any dev `runtime/models/`. The largest
/// `.bin` wins so a better model is preferred when several are present.
pub fn find_model(exe_dir: &Path, data_dir: &Path, setting: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = setting {
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    if let Some(p) = best_model_in(&models_dir(data_dir)) {
        return Some(p);
    }
    for up in ["", "..", "../..", "../../.."] {
        let base = exe_dir.join(up);
        if let Some(p) = best_model_in(&base.join("models")) {
            return Some(p);
        }
        if let Some(p) = best_model_in(&base.join("runtime").join("models")) {
            return Some(p);
        }
    }
    None
}

fn best_model_in(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(u64, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("bin") {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
                best = Some((size, path));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Download `url` to `dest` atomically, verifying `expected_sha256`.
pub fn download_file(
    url: &str,
    dest: &Path,
    expected_sha256: Option<&str>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), RuntimeError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("part");
    let response = ureq::get(url)
        .call()
        .map_err(|e| RuntimeError::Http(e.to_string()))?;
    let total: u64 = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut reader = response.into_reader();
    let mut file = fs::File::create(&tmp)?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done: u64 = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&tmp);
            return Err(RuntimeError::Cancelled);
        }
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        file.write_all(&buf[..read])?;
        done += read as u64;
        progress(done, total);
    }
    file.flush()?;
    drop(file);

    if let Some(expected) = expected_sha256 {
        let actual = hash_file(&tmp).map_err(|e| RuntimeError::Http(e.to_string()))?;
        if !actual.as_str().eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&tmp);
            return Err(RuntimeError::HashMismatch {
                expected: expected.to_string(),
                actual: actual.as_str().to_string(),
            });
        }
    }
    fs::rename(&tmp, dest)?;
    Ok(())
}

/// Download and extract the whisper.cpp Windows CPU runtime.
pub fn install_engine(
    data_dir: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&'static str, u64, u64),
) -> Result<PathBuf, RuntimeError> {
    let dir = engine_dir(data_dir);
    fs::create_dir_all(&dir)?;
    let zip_path = runtime_dir(data_dir).join("whisper-bin-x64.zip");
    download_file(
        ENGINE_ZIP_URL,
        &zip_path,
        Some(ENGINE_ZIP_SHA256),
        cancel,
        |d, t| progress("engine-download", d, t),
    )?;

    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| RuntimeError::Zip(e.to_string()))?;
    let total = archive.len() as u64;
    for index in 0..archive.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err(RuntimeError::Cancelled);
        }
        let mut entry = archive
            .by_index(index)
            .map_err(|e| RuntimeError::Zip(e.to_string()))?;
        let name = entry.name().to_string();
        let base = name.rsplit('/').next().unwrap_or(&name).to_string();
        let lower = base.to_ascii_lowercase();
        let keep = lower == "whisper-cli.exe" || lower.ends_with(".dll");
        if keep && !entry.is_dir() {
            let out = dir.join(&base);
            let mut out_file = fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
        progress("engine-extract", index as u64 + 1, total);
    }
    let _ = fs::remove_file(&zip_path);

    let binary = engine_binary(data_dir);
    if binary.is_file() {
        Ok(binary)
    } else {
        Err(RuntimeError::EngineMissing)
    }
}

/// Download a known model by id.
pub fn install_model(
    data_dir: &Path,
    id: &str,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&'static str, u64, u64),
) -> Result<PathBuf, RuntimeError> {
    let spec = model_spec(id).ok_or_else(|| RuntimeError::UnknownModel(id.to_string()))?;
    let dest = models_dir(data_dir).join(spec.file);
    download_file(spec.url, &dest, Some(spec.sha256), cancel, |d, t| {
        progress("model-download", d, t)
    })?;
    Ok(dest)
}

/// Minimal `which` for the Windows `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string())
        .split(';')
        .map(|s| s.to_string())
        .collect();
    for dir in std::env::split_paths(&path_var) {
        for ext in &exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
    }
    None
}
