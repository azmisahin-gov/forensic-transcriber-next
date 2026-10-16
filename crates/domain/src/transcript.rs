//! Transcript aggregate, segments and speaker labels.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{EvidenceId, SegmentId, TranscriptId};
use crate::time::Timestamp;

/// Review status of a segment.
///
/// `Automatic` is machine output and is **never** treated as expert output.
/// The four values mirror the source project so a reviewer sees the same
/// distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SegmentStatus {
    /// Machine output from the ASR engine.
    Automatic,
    /// A human checked it against the audio.
    Reviewed,
    /// A human changed the text, speaker or timing.
    Edited,
    /// A human explicitly confirmed it.
    Verified,
}

impl SegmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SegmentStatus::Automatic => "AUTOMATIC",
            SegmentStatus::Reviewed => "REVIEWED",
            SegmentStatus::Edited => "EDITED",
            SegmentStatus::Verified => "VERIFIED",
        }
    }

    /// Whether this status represents human work (as opposed to raw machine
    /// output).
    pub fn is_human(self) -> bool {
        !matches!(self, SegmentStatus::Automatic)
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value.to_ascii_uppercase().as_str() {
            "AUTOMATIC" => Ok(SegmentStatus::Automatic),
            "REVIEWED" => Ok(SegmentStatus::Reviewed),
            "EDITED" => Ok(SegmentStatus::Edited),
            "VERIFIED" => Ok(SegmentStatus::Verified),
            other => Err(DomainError::InvalidEnumValue {
                field: "segment_status",
                value: other.to_string(),
            }),
        }
    }
}

/// A speaker label assigned by the operator (e.g. `SPEAKER_01`). A label is a
/// workspace convention, **not** a biometric identity determination.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeakerLabel(String);

impl SpeakerLabel {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into().trim().to_string();
        if value.is_empty() {
            return Err(DomainError::EmptySpeakerLabel);
        }
        Ok(SpeakerLabel(value))
    }

    /// The default label used before a human reassigns speech.
    pub fn default_speaker() -> Self {
        SpeakerLabel("SPEAKER_01".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One time-aligned transcript segment.
///
/// Timing is stored in integer milliseconds. This removes the floating-point
/// rounding ambiguity that creeps in when seconds are stored as `f64` and
/// repeatedly rescaled. Timing validation only enforces `0 <= start < end`;
/// overlapping segments are allowed because real conversations have overlap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: SegmentId,
    pub transcript_id: TranscriptId,
    pub ordinal: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub speaker: SpeakerLabel,
    /// Current (possibly edited) text.
    pub text: String,
    /// The text exactly as the ASR engine produced it. Written once and never
    /// overwritten, so an expert edit can never destroy the automatic output.
    pub original_text: String,
    pub status: SegmentStatus,
    /// Engine confidence in [0, 1] when available.
    pub confidence: Option<f32>,
    /// Operator working flags such as `UNCLEAR`. Not a status change.
    pub flags: Vec<String>,
    pub updated_at: Timestamp,
}

impl TranscriptSegment {
    /// Create an automatic (machine) segment.
    pub fn automatic(
        transcript_id: TranscriptId,
        ordinal: u32,
        start_ms: i64,
        end_ms: i64,
        speaker: SpeakerLabel,
        text: impl Into<String>,
    ) -> Result<Self, DomainError> {
        Self::validate_timing(start_ms, end_ms)?;
        let text = text.into();
        Ok(Self {
            id: SegmentId::new(),
            transcript_id,
            ordinal,
            start_ms,
            end_ms,
            speaker,
            original_text: text.clone(),
            text,
            status: SegmentStatus::Automatic,
            confidence: None,
            flags: Vec::new(),
            updated_at: Timestamp::now(),
        })
    }

    /// Enforce non-negative, ordered timing.
    pub fn validate_timing(start_ms: i64, end_ms: i64) -> Result<(), DomainError> {
        if start_ms < 0 {
            return Err(DomainError::NegativeTiming(start_ms));
        }
        if start_ms >= end_ms {
            return Err(DomainError::InvalidSegmentTiming { start_ms, end_ms });
        }
        Ok(())
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> i64 {
        self.end_ms - self.start_ms
    }

    /// True when the segment text differs from the machine output.
    pub fn is_text_modified(&self) -> bool {
        self.text != self.original_text
    }

    /// Apply an expert text edit. The original machine text is preserved, the
    /// status changes to `Edited` (unless it was already `Verified`, which is
    /// never downgraded silently), and the timestamp is bumped.
    pub fn edit_text(&mut self, new_text: impl Into<String>) -> Result<(), DomainError> {
        let new_text = new_text.into();
        if new_text != self.text && self.status != SegmentStatus::Verified {
            self.status = SegmentStatus::Edited;
        }
        self.text = new_text;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    /// Reassign the speaker.
    pub fn set_speaker(&mut self, speaker: SpeakerLabel) {
        self.speaker = speaker;
        if self.status == SegmentStatus::Automatic {
            self.status = SegmentStatus::Edited;
        }
        self.updated_at = Timestamp::now();
    }

    /// Split this segment at `at_ms` into (self, new right-hand segment).
    /// The caller is responsible for inserting the returned segment and
    /// renumbering ordinals.
    pub fn split_at(
        &mut self,
        at_ms: i64,
        new_id: SegmentId,
    ) -> Result<TranscriptSegment, DomainError> {
        if at_ms <= self.start_ms || at_ms >= self.end_ms {
            return Err(DomainError::InvalidSegmentTiming {
                start_ms: at_ms,
                end_ms: self.end_ms,
            });
        }
        let old_end = self.end_ms;
        let right = TranscriptSegment {
            id: new_id,
            transcript_id: self.transcript_id.clone(),
            ordinal: self.ordinal,
            start_ms: at_ms,
            end_ms: old_end,
            speaker: self.speaker.clone(),
            text: String::new(),
            original_text: String::new(),
            status: SegmentStatus::Edited,
            confidence: None,
            flags: Vec::new(),
            updated_at: Timestamp::now(),
        };
        self.end_ms = at_ms;
        if self.status == SegmentStatus::Automatic {
            self.status = SegmentStatus::Edited;
        }
        self.updated_at = Timestamp::now();
        Ok(right)
    }
}

/// A transcript ties an ordered set of segments to one evidence file and
/// records which engine and model produced the automatic output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    pub id: TranscriptId,
    pub evidence_id: EvidenceId,
    pub language: String,
    pub engine: Option<String>,
    pub model_id: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Transcript {
    pub fn new(evidence_id: EvidenceId, language: impl Into<String>) -> Self {
        let now = Timestamp::now();
        Self {
            id: TranscriptId::new(),
            evidence_id,
            language: language.into(),
            engine: None,
            model_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Number of segments that still carry machine-only output.
    pub fn automatic_count(segments: &[TranscriptSegment]) -> usize {
        segments
            .iter()
            .filter(|s| s.status == SegmentStatus::Automatic)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: i64, end: i64, text: &str) -> TranscriptSegment {
        TranscriptSegment::automatic(
            TranscriptId::new(),
            0,
            start,
            end,
            SpeakerLabel::default_speaker(),
            text,
        )
        .unwrap()
    }

    #[test]
    fn rejects_inverted_timing() {
        let err = TranscriptSegment::automatic(
            TranscriptId::new(),
            0,
            500,
            400,
            SpeakerLabel::default_speaker(),
            "x",
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::InvalidSegmentTiming { .. }));
    }

    #[test]
    fn rejects_negative_start() {
        let err = TranscriptSegment::automatic(
            TranscriptId::new(),
            0,
            -1,
            100,
            SpeakerLabel::default_speaker(),
            "x",
        )
        .unwrap_err();
        assert_eq!(err, DomainError::NegativeTiming(-1));
    }

    #[test]
    fn edit_preserves_original_text() {
        let mut s = seg(0, 1000, "orijinal");
        s.edit_text("düzeltilmiş").unwrap();
        assert_eq!(s.text, "düzeltilmiş");
        assert_eq!(s.original_text, "orijinal");
        assert_eq!(s.status, SegmentStatus::Edited);
        assert!(s.is_text_modified());
    }

    #[test]
    fn edit_does_not_downgrade_verified() {
        let mut s = seg(0, 1000, "a");
        s.status = SegmentStatus::Verified;
        s.edit_text("b").unwrap();
        assert_eq!(s.status, SegmentStatus::Verified);
    }

    #[test]
    fn split_requires_interior_point() {
        let mut s = seg(1000, 2000, "hello");
        assert!(s.split_at(1000, SegmentId::new()).is_err());
        assert!(s.split_at(2000, SegmentId::new()).is_err());
        let right = s.split_at(1500, SegmentId::new()).unwrap();
        assert_eq!(s.end_ms, 1500);
        assert_eq!(right.start_ms, 1500);
        assert_eq!(right.end_ms, 2000);
        assert_eq!(s.status, SegmentStatus::Edited);
    }

    #[test]
    fn status_round_trips() {
        for s in [
            SegmentStatus::Automatic,
            SegmentStatus::Reviewed,
            SegmentStatus::Edited,
            SegmentStatus::Verified,
        ] {
            assert_eq!(SegmentStatus::parse(s.as_str()).unwrap(), s);
        }
        assert!(SegmentStatus::parse("BOGUS").is_err());
    }

    #[test]
    fn automatic_count_counts_only_machine() {
        let mut a = seg(0, 1, "a");
        let b = seg(1, 2, "b");
        a.edit_text("edited").unwrap();
        assert_eq!(Transcript::automatic_count(&[a, b]), 1);
    }
}
