//! `ftn-integrity` — hashing and safe file handling.
//!
//! The rules here implement the reliability priority of the product: an
//! original file is treated as read-only evidence and a copy is only accepted
//! after its SHA-256 has been re-read and matched.

use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use ftn_domain::evidence::Sha256;
use sha2::{Digest, Sha256 as Sha256Hasher};
use thiserror::Error;

/// Errors from integrity operations. These are safe to log; they never embed
/// file contents.
#[derive(Debug, Error)]
pub enum IntegrityError {
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("sha256 mismatch after copy: source={expected}, copy={actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("refusing path that escapes the destination directory: {0}")]
    PathTraversal(String),

    #[error("destination already exists: {0}")]
    DestinationExists(PathBuf),

    #[error("domain error: {0}")]
    Domain(#[from] ftn_domain::DomainError),
}

/// The chunk size used when streaming a file through the hasher (1 MiB). Large
/// files are never loaded fully into memory.
pub const HASH_CHUNK_BYTES: usize = 1024 * 1024;

/// Compute the SHA-256 of a file by streaming it in fixed-size chunks.
pub fn hash_file(path: &Path) -> Result<Sha256, IntegrityError> {
    let file = File::open(path).map_err(|e| IntegrityError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut reader = BufReader::with_capacity(HASH_CHUNK_BYTES, file);
    let mut hasher = Sha256Hasher::new();
    let mut buf = vec![0u8; HASH_CHUNK_BYTES];
    loop {
        let read = reader.read(&mut buf).map_err(|e| IntegrityError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let digest = hex_encode(&hasher.finalize());
    Ok(Sha256::parse(digest)?)
}

/// Compute the SHA-256 of an in-memory byte slice.
pub fn hash_bytes(bytes: &[u8]) -> Result<Sha256, IntegrityError> {
    let mut hasher = Sha256Hasher::new();
    hasher.update(bytes);
    let digest = hex_encode(&hasher.finalize());
    Ok(Sha256::parse(digest)?)
}

/// Reject any file name that is not a single, plain path component. This defends
/// against path traversal when a name comes from user input or an archive.
pub fn safe_file_name(name: &str) -> Result<String, IntegrityError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return Err(IntegrityError::PathTraversal(name.to_string()));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(IntegrityError::PathTraversal(name.to_string()));
    }
    // Windows drive-relative names like "C:" are not valid components either.
    if trimmed.contains(':') {
        return Err(IntegrityError::PathTraversal(name.to_string()));
    }
    Ok(trimmed.to_string())
}

/// Make a file name unique within a directory by appending ` (n)` before the
/// extension. The original is never overwritten.
pub fn unique_file_name(dir: &Path, name: &str) -> Result<String, IntegrityError> {
    let name = safe_file_name(name)?;
    if !dir.join(&name).exists() {
        return Ok(name);
    }
    let path = Path::new(&name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str());
    for n in 1..10_000u32 {
        let candidate = match ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        if !dir.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(IntegrityError::DestinationExists(dir.join(name)))
}

/// The result of a verified copy.
#[derive(Debug, Clone)]
pub struct VerifiedCopy {
    pub dest_path: PathBuf,
    pub source_sha256: Sha256,
    pub copied_sha256: Sha256,
    pub bytes: u64,
}

/// Copy `source` into `dest_dir` under `dest_name`, then re-hash the copy and
/// confirm it matches the source. The copy is written to a temporary file first
/// and renamed into place only on success, so an interrupted copy never leaves a
/// half-written file that looks complete.
///
/// `progress` is called with the number of bytes copied so far.
pub fn copy_and_verify<F>(
    source: &Path,
    dest_dir: &Path,
    dest_name: &str,
    mut progress: F,
) -> Result<VerifiedCopy, IntegrityError>
where
    F: FnMut(u64),
{
    let dest_name = safe_file_name(dest_name)?;
    fs::create_dir_all(dest_dir).map_err(|e| IntegrityError::Io {
        path: dest_dir.to_path_buf(),
        source: e,
    })?;

    let source_sha256 = hash_file(source)?;
    let dest_path = dest_dir.join(&dest_name);
    let tmp_path = dest_dir.join(format!(".{dest_name}.part"));

    let bytes = copy_streaming(source, &tmp_path, &mut progress)?;
    let copied_sha256 = hash_file(&tmp_path)?;

    if copied_sha256 != source_sha256 {
        let _ = fs::remove_file(&tmp_path);
        return Err(IntegrityError::HashMismatch {
            expected: source_sha256.as_str().to_string(),
            actual: copied_sha256.as_str().to_string(),
        });
    }

    // Atomic-ish publish: rename the verified temp file into place.
    fs::rename(&tmp_path, &dest_path).map_err(|e| IntegrityError::Io {
        path: dest_path.clone(),
        source: e,
    })?;

    Ok(VerifiedCopy {
        dest_path,
        source_sha256,
        copied_sha256,
        bytes,
    })
}

fn copy_streaming<F>(source: &Path, dest: &Path, progress: &mut F) -> Result<u64, IntegrityError>
where
    F: FnMut(u64),
{
    let mut reader = BufReader::with_capacity(
        HASH_CHUNK_BYTES,
        File::open(source).map_err(|e| IntegrityError::Io {
            path: source.to_path_buf(),
            source: e,
        })?,
    );
    let mut writer = File::create(dest).map_err(|e| IntegrityError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;
    let mut buf = vec![0u8; HASH_CHUNK_BYTES];
    let mut total: u64 = 0;
    loop {
        let read = reader.read(&mut buf).map_err(|e| IntegrityError::Io {
            path: source.to_path_buf(),
            source: e,
        })?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buf[..read])
            .map_err(|e| IntegrityError::Io {
                path: dest.to_path_buf(),
                source: e,
            })?;
        total += read as u64;
        progress(total);
    }
    writer.flush().map_err(|e| IntegrityError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;
    Ok(total)
}

/// Re-hash a stored copy and compare it with the digest recorded at import time.
pub fn verify_against(path: &Path, expected: &Sha256) -> Result<Sha256, IntegrityError> {
    let actual = hash_file(path)?;
    if &actual != expected {
        return Err(IntegrityError::HashMismatch {
            expected: expected.as_str().to_string(),
            actual: actual.as_str().to_string(),
        });
    }
    Ok(actual)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ftn-integrity-{}", uuid_like()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn uuid_like() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{n}")
    }

    fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let p = dir.join(name);
        let mut f = File::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    #[test]
    fn known_sha256_for_abc() {
        // SHA-256("abc") is a well-known constant.
        let h = hash_bytes(b"abc").unwrap();
        assert_eq!(
            h.as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rejects_path_traversal_names() {
        assert!(safe_file_name("../etc/passwd").is_err());
        assert!(safe_file_name("..\\windows\\system32").is_err());
        assert!(safe_file_name("a/b").is_err());
        assert!(safe_file_name("C:evil").is_err());
        assert!(safe_file_name("").is_err());
        assert!(safe_file_name("..").is_err());
        assert_eq!(safe_file_name(" kayit.wav ").unwrap(), "kayit.wav");
    }

    #[test]
    fn unique_name_avoids_collision() {
        let dir = tmpdir();
        write_file(&dir, "a.wav", b"1");
        assert_eq!(unique_file_name(&dir, "a.wav").unwrap(), "a (1).wav");
        assert_eq!(unique_file_name(&dir, "b.wav").unwrap(), "b.wav");
    }

    #[test]
    fn copy_and_verify_succeeds_and_reports_progress() {
        let dir = tmpdir();
        let src = write_file(&dir, "src.bin", &vec![7u8; 3 * 1024 * 1024]);
        let dest_dir = dir.join("store");
        let mut calls = 0u64;
        let result = copy_and_verify(&src, &dest_dir, "copy.bin", |_| calls += 1).unwrap();
        assert_eq!(result.bytes, 3 * 1024 * 1024);
        assert_eq!(result.source_sha256, result.copied_sha256);
        assert!(result.dest_path.exists());
        assert!(
            calls >= 3,
            "expected at least 3 progress calls, got {calls}"
        );
        // The temp file must not be left behind.
        assert!(!dest_dir.join(".copy.bin.part").exists());
    }

    #[test]
    fn verify_against_detects_mismatch() {
        let dir = tmpdir();
        let src = write_file(&dir, "src.bin", b"hello");
        let wrong = Sha256::parse("a".repeat(64)).unwrap();
        let err = verify_against(&src, &wrong).unwrap_err();
        assert!(matches!(err, IntegrityError::HashMismatch { .. }));
    }

    #[test]
    fn verify_against_accepts_correct_hash() {
        let dir = tmpdir();
        let src = write_file(&dir, "src.bin", b"hello");
        let ok = hash_file(&src).unwrap();
        assert_eq!(verify_against(&src, &ok).unwrap(), ok);
    }
}
