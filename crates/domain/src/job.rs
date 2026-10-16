//! Transcription jobs — one durable record per attempt.

use serde::{Deserialize, Serialize};

use crate::ids::{EvidenceId, JobId, TranscriptId};
use crate::time::Timestamp;

/// Lifecycle of a transcription job. Queued, running, succeeded, failed and
/// cancelled are distinct: a partially-written job is never presented as a
/// completed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "QUEUED",
            JobStatus::Running => "RUNNING",
            JobStatus::Succeeded => "SUCCEEDED",
            JobStatus::Failed => "FAILED",
            JobStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "QUEUED" => Some(JobStatus::Queued),
            "RUNNING" => Some(JobStatus::Running),
            "SUCCEEDED" => Some(JobStatus::Succeeded),
            "FAILED" => Some(JobStatus::Failed),
            "CANCELLED" => Some(JobStatus::Cancelled),
            _ => None,
        }
    }

    /// Whether the job has reached a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Succeeded | JobStatus::Failed | JobStatus::Cancelled
        )
    }
}

/// Snapshot of the engine that produced (or attempted) a transcript.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EngineInfo {
    /// Adapter identifier, e.g. `whisper.cpp`.
    pub engine: String,
    /// Engine version string as reported by the executable.
    pub engine_version: Option<String>,
    /// Model identifier, e.g. `ggml-large-v3-turbo-q5_0`.
    pub model_id: Option<String>,
    pub model_sha256: Option<String>,
    /// Serialised run parameters (language, VAD, threads, ...).
    pub settings: serde_json::Value,
}

/// One durable record per transcription attempt, success or failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionJob {
    pub id: JobId,
    pub evidence_id: EvidenceId,
    pub transcript_id: Option<TranscriptId>,
    pub status: JobStatus,
    pub engine: Option<EngineInfo>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
}

impl TranscriptionJob {
    /// Create a queued job.
    pub fn queued(evidence_id: EvidenceId) -> Self {
        Self {
            id: JobId::new(),
            evidence_id,
            transcript_id: None,
            status: JobStatus::Queued,
            engine: None,
            error_code: None,
            error_message: None,
            started_at: Timestamp::now(),
            finished_at: None,
        }
    }

    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Timestamp::now();
    }

    pub fn succeed(&mut self, transcript_id: TranscriptId) {
        self.status = JobStatus::Succeeded;
        self.transcript_id = Some(transcript_id);
        self.finished_at = Some(Timestamp::now());
    }

    /// Mark as failed with a stable error code (not a raw stack trace).
    pub fn fail(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.status = JobStatus::Failed;
        self.error_code = Some(code.into());
        self.error_message = Some(message.into());
        self.finished_at = Some(Timestamp::now());
    }

    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.finished_at = Some(Timestamp::now());
    }

    /// Mark as cancelled with an explicit code and message (distinct from a
    /// failure, so the UI and audit trail keep the two apart).
    pub fn cancel_with(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.status = JobStatus::Cancelled;
        self.error_code = Some(code.into());
        self.error_message = Some(message.into());
        self.finished_at = Some(Timestamp::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips() {
        for s in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Succeeded,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ] {
            assert_eq!(JobStatus::parse(s.as_str()), Some(s));
        }
    }

    #[test]
    fn terminal_states_are_exclusive() {
        assert!(!JobStatus::Queued.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Succeeded.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn failure_records_code_and_finish_time() {
        let mut j = TranscriptionJob::queued(EvidenceId::new());
        j.fail("ENGINE_NOT_FOUND", "whisper.cpp executable not found");
        assert_eq!(j.status, JobStatus::Failed);
        assert_eq!(j.error_code.as_deref(), Some("ENGINE_NOT_FOUND"));
        assert!(j.finished_at.is_some());
    }
}
