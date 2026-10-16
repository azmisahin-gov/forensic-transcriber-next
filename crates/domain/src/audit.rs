//! Audit / provenance events.
//!
//! These records are described as **software provenance and traceability
//! features**. They are not a legal chain of custody and make no such claim.

use serde::{Deserialize, Serialize};

use crate::ids::{AuditEventId, CaseId, EvidenceId, SegmentId, TranscriptId};
use crate::time::Timestamp;

/// The kind of change an audit event describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    CaseCreated,
    CaseRenamed,
    EvidenceImported,
    EvidenceIntegrityChecked,
    TranscribeStarted,
    TranscribeSucceeded,
    TranscribeFailed,
    TranscribeCancelled,
    SegmentTextEdited,
    SegmentSpeakerChanged,
    SegmentSplit,
    SegmentMerged,
    SegmentStatusChanged,
    TranscriptSaved,
    RevisionCreated,
    ExportCreated,
}

impl AuditAction {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditAction::CaseCreated => "CASE_CREATED",
            AuditAction::CaseRenamed => "CASE_RENAMED",
            AuditAction::EvidenceImported => "EVIDENCE_IMPORTED",
            AuditAction::EvidenceIntegrityChecked => "EVIDENCE_INTEGRITY_CHECKED",
            AuditAction::TranscribeStarted => "TRANSCRIBE_STARTED",
            AuditAction::TranscribeSucceeded => "TRANSCRIBE_SUCCEEDED",
            AuditAction::TranscribeFailed => "TRANSCRIBE_FAILED",
            AuditAction::TranscribeCancelled => "TRANSCRIBE_CANCELLED",
            AuditAction::SegmentTextEdited => "SEGMENT_TEXT_EDITED",
            AuditAction::SegmentSpeakerChanged => "SEGMENT_SPEAKER_CHANGED",
            AuditAction::SegmentSplit => "SEGMENT_SPLIT",
            AuditAction::SegmentMerged => "SEGMENT_MERGED",
            AuditAction::SegmentStatusChanged => "SEGMENT_STATUS_CHANGED",
            AuditAction::TranscriptSaved => "TRANSCRIPT_SAVED",
            AuditAction::RevisionCreated => "REVISION_CREATED",
            AuditAction::ExportCreated => "EXPORT_CREATED",
        }
    }
}

/// A single audit event. `before` and `after` hold the relevant values so a
/// reviewer can see exactly what changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: AuditEventId,
    pub case_id: CaseId,
    pub evidence_id: Option<EvidenceId>,
    pub transcript_id: Option<TranscriptId>,
    pub segment_id: Option<SegmentId>,
    pub action: AuditAction,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    /// Free-text label supplied by the operator, if any. This is **not** a
    /// verified identity; the application has no authentication.
    pub actor_label: Option<String>,
    pub app_version: String,
    pub created_at: Timestamp,
}

impl AuditEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(case_id: CaseId, action: AuditAction, app_version: impl Into<String>) -> Self {
        Self {
            id: AuditEventId::new(),
            case_id,
            evidence_id: None,
            transcript_id: None,
            segment_id: None,
            action,
            before: None,
            after: None,
            actor_label: None,
            app_version: app_version.into(),
            created_at: Timestamp::now(),
        }
    }

    pub fn with_evidence(mut self, evidence_id: EvidenceId) -> Self {
        self.evidence_id = Some(evidence_id);
        self
    }

    pub fn with_transcript(mut self, transcript_id: TranscriptId) -> Self {
        self.transcript_id = Some(transcript_id);
        self
    }

    pub fn with_segment(mut self, segment_id: SegmentId) -> Self {
        self.segment_id = Some(segment_id);
        self
    }

    pub fn with_change(
        mut self,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    ) -> Self {
        self.before = before;
        self.after = after;
        self
    }

    pub fn with_actor(mut self, label: impl Into<String>) -> Self {
        self.actor_label = Some(label.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_links() {
        let case_id = CaseId::new();
        let ev = AuditEvent::new(case_id.clone(), AuditAction::CaseCreated, "0.1.0")
            .with_actor("Uzman A")
            .with_change(None, Some(serde_json::json!({"title": "Vaka"})));
        assert_eq!(ev.case_id, case_id);
        assert_eq!(ev.action.as_str(), "CASE_CREATED");
        assert!(ev.after.is_some());
        assert_eq!(ev.actor_label.as_deref(), Some("Uzman A"));
    }
}
