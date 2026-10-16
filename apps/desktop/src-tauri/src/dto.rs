//! Serialisable data-transfer objects for the Tauri command surface.
//!
//! These use camelCase for JavaScript consumption and render timestamps as
//! ISO-8601 strings. No domain type is exposed directly across the IPC boundary.

use ftn_domain::case::Case;
use ftn_domain::evidence::EvidenceFile;
use ftn_domain::export_record::ExportRecord;
use ftn_domain::job::TranscriptionJob;
use ftn_domain::transcript::{Transcript, TranscriptSegment};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseDto {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Case> for CaseDto {
    fn from(c: &Case) -> Self {
        Self {
            id: c.id.to_string(),
            title: c.title.clone(),
            notes: c.notes.clone(),
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadataDto {
    pub format: Option<String>,
    pub codec: Option<String>,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bit_depth: Option<u32>,
    pub audio_stream_count: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDto {
    pub id: String,
    pub case_id: String,
    pub original_name: String,
    pub stored_name: String,
    pub original_path: String,
    pub stored_path: String,
    pub size_bytes: u64,
    pub metadata: MediaMetadataDto,
    pub source_sha256: String,
    pub stored_sha256: String,
    pub integrity: String,
    pub imported_at: String,
}

impl From<&EvidenceFile> for EvidenceDto {
    fn from(e: &EvidenceFile) -> Self {
        Self {
            id: e.id.to_string(),
            case_id: e.case_id.to_string(),
            original_name: e.original_name.clone(),
            stored_name: e.stored_name.clone(),
            original_path: e.original_path.clone(),
            stored_path: e.stored_path.clone(),
            size_bytes: e.size_bytes,
            metadata: MediaMetadataDto {
                format: e.metadata.format.clone(),
                codec: e.metadata.codec.clone(),
                duration_seconds: e.metadata.duration_seconds,
                sample_rate: e.metadata.sample_rate,
                channels: e.metadata.channels,
                bit_depth: e.metadata.bit_depth,
                audio_stream_count: e.metadata.audio_stream_count,
            },
            source_sha256: e.source_sha256.as_str().to_string(),
            stored_sha256: e.stored_sha256.as_str().to_string(),
            integrity: format!("{:?}", e.integrity).to_uppercase(),
            imported_at: e.imported_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDto {
    pub id: String,
    pub transcript_id: String,
    pub ordinal: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub speaker: String,
    pub text: String,
    pub original_text: String,
    pub status: String,
    pub confidence: Option<f32>,
    pub flags: Vec<String>,
    pub updated_at: String,
}

impl From<&TranscriptSegment> for SegmentDto {
    fn from(s: &TranscriptSegment) -> Self {
        Self {
            id: s.id.to_string(),
            transcript_id: s.transcript_id.to_string(),
            ordinal: s.ordinal,
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            speaker: s.speaker.as_str().to_string(),
            text: s.text.clone(),
            original_text: s.original_text.clone(),
            status: s.status.as_str().to_string(),
            confidence: s.confidence,
            flags: s.flags.clone(),
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptDto {
    pub id: String,
    pub evidence_id: String,
    pub language: String,
    pub engine: Option<String>,
    pub model_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Transcript> for TranscriptDto {
    fn from(t: &Transcript) -> Self {
        Self {
            id: t.id.to_string(),
            evidence_id: t.evidence_id.to_string(),
            language: t.language.clone(),
            engine: t.engine.clone(),
            model_id: t.model_id.clone(),
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub id: String,
    pub evidence_id: String,
    pub transcript_id: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

impl From<&TranscriptionJob> for JobDto {
    fn from(j: &TranscriptionJob) -> Self {
        Self {
            id: j.id.to_string(),
            evidence_id: j.evidence_id.to_string(),
            transcript_id: j.transcript_id.as_ref().map(|t| t.to_string()),
            status: j.status.as_str().to_string(),
            error_code: j.error_code.clone(),
            error_message: j.error_message.clone(),
            started_at: j.started_at.to_rfc3339(),
            finished_at: j.finished_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOutcomeDto {
    pub transcript: TranscriptDto,
    pub segments: Vec<SegmentDto>,
    pub job: JobDto,
    pub skipped_segments: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRecordDto {
    pub id: String,
    pub format: String,
    pub path: String,
    pub sha256: Option<String>,
    pub schema_version: String,
    pub created_at: String,
}

impl From<&ExportRecord> for ExportRecordDto {
    fn from(r: &ExportRecord) -> Self {
        Self {
            id: r.id.to_string(),
            format: r.format.as_str().to_string(),
            path: r.path.clone(),
            sha256: r.sha256.clone(),
            schema_version: r.schema_version.clone(),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineProbeDto {
    pub available: bool,
    pub version: Option<String>,
    pub note: Option<String>,
}
