//! Storage errors.

use thiserror::Error;

/// Errors raised by the persistence layer. They never contain transcript text
/// or audio content, so they are safe to log.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("domain error: {0}")]
    Domain(#[from] ftn_domain::DomainError),

    #[error("not found: {entity} {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("a transcript already exists for evidence {0}")]
    TranscriptExists(String),

    #[error("schema version {found} is newer than this build supports ({supported})")]
    SchemaTooNew { found: i64, supported: i64 },
}
