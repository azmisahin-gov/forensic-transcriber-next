//! Immutable transcript revisions.
//!
//! A revision is a complete snapshot of a transcript's segments. A new ASR run
//! or an explicit save appends a revision instead of overwriting the previous
//! one, so reviewed/edited/verified human work is never silently destroyed.
//! Exactly one revision per transcript is current.

use serde::{Deserialize, Serialize};

use crate::ids::{JobId, RevisionId, TranscriptId};
use crate::time::Timestamp;
use crate::transcript::TranscriptSegment;

/// The nature of a revision snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionState {
    /// Raw machine output straight from the engine.
    Machine,
    /// Human review pass.
    Reviewed,
    /// Human edits applied.
    Edited,
    /// Human confirmation.
    Verified,
}

impl RevisionState {
    pub fn as_str(self) -> &'static str {
        match self {
            RevisionState::Machine => "MACHINE",
            RevisionState::Reviewed => "REVIEWED",
            RevisionState::Edited => "EDITED",
            RevisionState::Verified => "VERIFIED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "MACHINE" => Some(RevisionState::Machine),
            "REVIEWED" => Some(RevisionState::Reviewed),
            "EDITED" => Some(RevisionState::Edited),
            "VERIFIED" => Some(RevisionState::Verified),
            _ => None,
        }
    }
}

/// A frozen snapshot of a transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptRevision {
    pub id: RevisionId,
    pub transcript_id: TranscriptId,
    /// The job that produced this revision, when it came from an ASR run.
    pub job_id: Option<JobId>,
    pub state: RevisionState,
    pub is_current: bool,
    pub segments: Vec<TranscriptSegment>,
    pub created_at: Timestamp,
}

impl TranscriptRevision {
    pub fn new(
        transcript_id: TranscriptId,
        job_id: Option<JobId>,
        state: RevisionState,
        segments: Vec<TranscriptSegment>,
    ) -> Self {
        Self {
            id: RevisionId::new(),
            transcript_id,
            job_id,
            state,
            is_current: false,
            segments,
            created_at: Timestamp::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips() {
        for s in [
            RevisionState::Machine,
            RevisionState::Reviewed,
            RevisionState::Edited,
            RevisionState::Verified,
        ] {
            assert_eq!(RevisionState::parse(s.as_str()), Some(s));
        }
        assert_eq!(RevisionState::parse("nope"), None);
    }
}
