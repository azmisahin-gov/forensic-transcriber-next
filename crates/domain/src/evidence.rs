//! Evidence and derived media.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{CaseId, DerivedAssetId, EvidenceId};
use crate::time::Timestamp;

/// A validated SHA-256 digest, always stored as 64 lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256(String);

impl Sha256 {
    /// Parse and normalise a digest. Uppercase input is accepted and lowered.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DomainError> {
        let lower = value.as_ref().trim().to_ascii_lowercase();
        let ok = lower.len() == 64 && lower.bytes().all(|b| b.is_ascii_hexdigit());
        if !ok {
            return Err(DomainError::InvalidSha256(value.as_ref().to_string()));
        }
        Ok(Sha256(lower))
    }

    /// Borrow the normalised hex string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Outcome of re-hashing a stored copy and comparing it with the value recorded
/// at import time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrityStatus {
    /// The stored copy matches the recorded digest.
    Verified,
    /// The stored copy exists but its digest differs — possible tampering or
    /// corruption. This is never silently ignored.
    Mismatch,
    /// The stored copy is missing.
    Missing,
    /// The digest has not been recomputed in this session.
    Unchecked,
}

/// Technical metadata read from the media by FFprobe. Every field is optional:
/// an unreadable or unsupported container yields `None` rather than a guess.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub format: Option<String>,
    pub codec: Option<String>,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bit_depth: Option<u32>,
    pub audio_stream_count: Option<u32>,
}

impl MediaMetadata {
    /// Basic sanity check on decoded metadata. Durations must be finite and
    /// non-negative when present.
    pub fn is_sane(&self) -> bool {
        match self.duration_seconds {
            Some(d) => d.is_finite() && d >= 0.0,
            None => true,
        }
    }
}

/// An imported evidence file. The original user file is **never** modified; the
/// application keeps a verified copy plus the metadata needed to reason about
/// it later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceFile {
    pub id: EvidenceId,
    /// The case this evidence belongs to.
    pub case_id: CaseId,
    /// The name as it was on the user's disk.
    pub original_name: String,
    /// The name used inside the managed evidence store.
    pub stored_name: String,
    /// The path the user selected (informational only).
    pub original_path: String,
    /// Path of the verified working copy inside the case workspace.
    pub stored_path: String,
    pub size_bytes: u64,
    pub metadata: MediaMetadata,
    /// SHA-256 of the file as it was on the user's disk at import time.
    pub source_sha256: Sha256,
    /// SHA-256 recomputed from the managed copy; must equal `source_sha256`.
    pub stored_sha256: Sha256,
    pub integrity: IntegrityStatus,
    pub imported_at: Timestamp,
}

impl EvidenceFile {
    /// Whether the copy was verified against the source digest at import time.
    pub fn is_integrity_verified(&self) -> bool {
        self.integrity == IntegrityStatus::Verified && self.source_sha256 == self.stored_sha256
    }
}

/// A media file derived from an original evidence file (for example the 16 kHz
/// mono WAV used for transcription). Derivation parameters are recorded so the
/// transformation is reproducible and auditable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedMediaAsset {
    pub id: DerivedAssetId,
    pub evidence_id: EvidenceId,
    /// Relative path inside the case workspace.
    pub path: String,
    pub kind: DerivedKind,
    pub format: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sha256: Sha256,
    /// The exact tool and arguments used to produce this asset.
    pub generator: String,
    pub created_at: Timestamp,
}

/// The purpose of a derived asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DerivedKind {
    /// 16 kHz mono PCM WAV for the ASR engine.
    TranscriptionInput,
    /// A downsampled waveform summary for display.
    Waveform,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_accepts_upper_and_normalises() {
        let up = "A".repeat(64);
        let parsed = Sha256::parse(up).unwrap();
        assert_eq!(parsed.as_str(), &"a".repeat(64));
    }

    #[test]
    fn sha256_rejects_wrong_length() {
        assert!(Sha256::parse("abc").is_err());
    }

    #[test]
    fn sha256_rejects_non_hex() {
        let bad = "z".repeat(64);
        assert!(Sha256::parse(bad).is_err());
    }

    #[test]
    fn metadata_sanity_rejects_nan() {
        let m = MediaMetadata {
            duration_seconds: Some(f64::NAN),
            ..Default::default()
        };
        assert!(!m.is_sane());
    }

    #[test]
    fn integrity_verified_requires_matching_digests() {
        let d = Sha256::parse("a".repeat(64)).unwrap();
        let other = Sha256::parse("b".repeat(64)).unwrap();
        let ev = EvidenceFile {
            id: EvidenceId::new(),
            case_id: CaseId::new(),
            original_name: "x.wav".into(),
            stored_name: "x.wav".into(),
            original_path: "C:\\x.wav".into(),
            stored_path: "store/x.wav".into(),
            size_bytes: 10,
            metadata: MediaMetadata::default(),
            source_sha256: d.clone(),
            stored_sha256: other,
            integrity: IntegrityStatus::Verified,
            imported_at: Timestamp::from_millis(0),
        };
        assert!(!ev.is_integrity_verified());
    }
}
