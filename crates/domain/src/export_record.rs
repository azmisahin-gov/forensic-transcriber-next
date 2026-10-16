//! Export records and supported formats.

use serde::{Deserialize, Serialize};

use crate::ids::{CaseId, EvidenceId, ExportId, TranscriptId};
use crate::time::Timestamp;

/// A supported export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExportFormat {
    Txt,
    Srt,
    Json,
    Html,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Txt => "txt",
            ExportFormat::Srt => "srt",
            ExportFormat::Json => "json",
            ExportFormat::Html => "html",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ExportFormat::Txt => "TXT",
            ExportFormat::Srt => "SRT",
            ExportFormat::Json => "JSON",
            ExportFormat::Html => "HTML",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "TXT" => Some(ExportFormat::Txt),
            "SRT" => Some(ExportFormat::Srt),
            "JSON" => Some(ExportFormat::Json),
            "HTML" => Some(ExportFormat::Html),
            _ => None,
        }
    }
}

/// Record of a produced export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportRecord {
    pub id: ExportId,
    pub case_id: CaseId,
    pub transcript_id: Option<TranscriptId>,
    pub evidence_id: Option<EvidenceId>,
    pub format: ExportFormat,
    pub path: String,
    pub sha256: Option<String>,
    pub schema_version: String,
    pub created_at: Timestamp,
}

impl ExportRecord {
    pub fn new(
        case_id: CaseId,
        format: ExportFormat,
        path: impl Into<String>,
        schema_version: impl Into<String>,
    ) -> Self {
        Self {
            id: ExportId::new(),
            case_id,
            transcript_id: None,
            evidence_id: None,
            format,
            path: path.into(),
            sha256: None,
            schema_version: schema_version.into(),
            created_at: Timestamp::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_round_trips() {
        for f in [
            ExportFormat::Txt,
            ExportFormat::Srt,
            ExportFormat::Json,
            ExportFormat::Html,
        ] {
            assert_eq!(ExportFormat::parse(f.as_str()), Some(f));
            assert!(!f.extension().is_empty());
        }
    }
}
