//! Domain errors.

use thiserror::Error;

/// Errors raised by domain invariants. These are programming or data-integrity
/// errors, not user-facing messages; the application layer maps them into safe
/// messages.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("title must not be empty")]
    EmptyTitle,

    #[error("segment start ({start_ms} ms) must be less than end ({end_ms} ms)")]
    InvalidSegmentTiming { start_ms: i64, end_ms: i64 },

    #[error("segment timing must not be negative (start {0} ms)")]
    NegativeTiming(i64),

    #[error("evidence size must not be negative")]
    NegativeSize,

    #[error("sha256 digest must be 64 lowercase hex characters, got: {0}")]
    InvalidSha256(String),

    #[error("speaker label must not be empty")]
    EmptySpeakerLabel,

    #[error("invalid enum value for {field}: {value}")]
    InvalidEnumValue { field: &'static str, value: String },

    #[error("value out of range for {field}: {value}")]
    OutOfRange { field: &'static str, value: i64 },
}
