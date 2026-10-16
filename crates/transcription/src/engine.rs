//! The engine abstraction. Concrete engines implement [`EngineAdapter`] so the
//! pipeline never depends on a specific binary.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ftn_domain::ids::EvidenceId;
use thiserror::Error;

/// Errors from the transcription layer. Every variant maps to a stable code so
/// the UI never has to match on message strings.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine executable not found: {0}")]
    EngineNotFound(String),

    #[error("model file not found: {0}")]
    ModelNotFound(String),

    #[error("input audio not found: {0}")]
    InputNotFound(String),

    #[error("engine process failed (exit {code}): {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("could not parse engine output: {0}")]
    Parse(String),

    #[error("job was cancelled")]
    Cancelled,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl EngineError {
    /// A stable, machine-readable error code.
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::EngineNotFound(_) => "ENGINE_NOT_FOUND",
            EngineError::ModelNotFound(_) => "MODEL_NOT_FOUND",
            EngineError::InputNotFound(_) => "INPUT_NOT_FOUND",
            EngineError::ProcessFailed { .. } => "PROCESS_FAILED",
            EngineError::Parse(_) => "PARSE_ERROR",
            EngineError::Cancelled => "CANCELLED",
            EngineError::Io(_) => "IO_ERROR",
        }
    }
}

/// What the engine reports about itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EngineProbe {
    pub available: bool,
    pub version: Option<String>,
    /// Device/backend lines from the engine's own report, if any.
    pub devices: Vec<String>,
    pub note: Option<String>,
}

/// One raw segment as emitted by the engine (times in milliseconds).
#[derive(Debug, Clone, PartialEq)]
pub struct RawSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
}

/// A request to transcribe one audio file.
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    pub job_evidence_id: EvidenceId,
    pub input: PathBuf,
    pub model: PathBuf,
    pub language: String,
    pub threads: Option<u32>,
    pub use_gpu: bool,
}

/// The engine's output for one request.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionOutput {
    pub segments: Vec<RawSegment>,
    pub engine_version: Option<String>,
    pub model_id: Option<String>,
    pub settings: serde_json::Value,
}

/// A cooperative cancellation flag shared with the running process.
pub type CancelToken = Arc<AtomicBool>;

/// Progress callback: fraction in [0, 1] when known.
pub type ProgressFn<'a> = &'a mut dyn FnMut(Option<f32>);

/// A transcription engine.
pub trait EngineAdapter: Send + Sync {
    /// Stable identifier, e.g. `whisper.cpp`.
    fn id(&self) -> &'static str;

    /// Report whether the engine can run, without transcribing.
    fn probe(&self) -> EngineProbe;

    /// Transcribe `request`, updating `progress` and honouring `cancel`.
    fn transcribe(
        &self,
        request: &TranscriptionRequest,
        cancel: &CancelToken,
        progress: ProgressFn<'_>,
    ) -> Result<TranscriptionOutput, EngineError>;
}

/// Convenience constructor for a request.
impl TranscriptionRequest {
    pub fn new(
        evidence_id: EvidenceId,
        input: impl Into<PathBuf>,
        model: impl Into<PathBuf>,
    ) -> Self {
        Self {
            job_evidence_id: evidence_id,
            input: input.into(),
            model: model.into(),
            language: "tr".to_string(),
            threads: None,
            use_gpu: false,
        }
    }

    /// Validate the inputs exist before launching a process.
    pub fn validate(&self, engine_path: Option<&Path>) -> Result<(), EngineError> {
        if let Some(p) = engine_path {
            if !p.is_file() {
                return Err(EngineError::EngineNotFound(p.display().to_string()));
            }
        }
        if !self.model.is_file() {
            return Err(EngineError::ModelNotFound(self.model.display().to_string()));
        }
        if !self.input.is_file() {
            return Err(EngineError::InputNotFound(self.input.display().to_string()));
        }
        Ok(())
    }
}
