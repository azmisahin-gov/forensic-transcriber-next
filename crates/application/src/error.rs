//! Application-layer errors. Every variant maps to a safe code and message the
//! UI can display without leaking stack traces.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("case not found: {0}")]
    CaseNotFound(String),

    #[error("evidence not found: {0}")]
    EvidenceNotFound(String),

    #[error("transcript not found: {0}")]
    TranscriptNotFound(String),

    #[error("segment not found: {0}")]
    SegmentNotFound(String),

    #[error("a transcript already exists for this evidence; choose a new revision instead of overwriting")]
    TranscriptExists,

    #[error("integrity check failed for evidence {id}: {detail}")]
    IntegrityFailed { id: String, detail: String },

    #[error("transcription engine is not available: {0}")]
    EngineUnavailable(String),

    #[error("media tool missing: {0}")]
    MediaToolMissing(String),

    #[error("domain error: {0}")]
    Domain(#[from] ftn_domain::DomainError),

    #[error("storage error: {0}")]
    Storage(#[from] ftn_storage::StorageError),

    #[error("integrity error: {0}")]
    Integrity(#[from] ftn_integrity::IntegrityError),

    #[error("media error: {0}")]
    Media(#[from] ftn_media::MediaError),

    #[error("engine error: {0}")]
    Engine(#[from] ftn_transcription::EngineError),

    #[error("export error: {0}")]
    Export(#[from] ftn_export::ExportError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// A stable, machine-readable code.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::CaseNotFound(_) => "CASE_NOT_FOUND",
            AppError::EvidenceNotFound(_) => "EVIDENCE_NOT_FOUND",
            AppError::TranscriptNotFound(_) => "TRANSCRIPT_NOT_FOUND",
            AppError::SegmentNotFound(_) => "SEGMENT_NOT_FOUND",
            AppError::TranscriptExists => "TRANSCRIPT_EXISTS",
            AppError::IntegrityFailed { .. } => "INTEGRITY_FAILED",
            AppError::EngineUnavailable(_) => "ENGINE_UNAVAILABLE",
            AppError::MediaToolMissing(_) => "MEDIA_TOOL_MISSING",
            AppError::Domain(_) => "DOMAIN_ERROR",
            AppError::Storage(_) => "STORAGE_ERROR",
            AppError::Integrity(_) => "INTEGRITY_ERROR",
            AppError::Media(_) => "MEDIA_ERROR",
            AppError::Engine(e) => e.code(),
            AppError::Export(_) => "EXPORT_ERROR",
            AppError::Io(_) => "IO_ERROR",
        }
    }
}
