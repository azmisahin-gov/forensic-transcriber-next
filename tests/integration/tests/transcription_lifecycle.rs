//! Tests for the split transcription lifecycle: begin → complete / fail. These
//! exercise the persistence phases directly (no engine run, no FFmpeg) so the
//! cancel/fail path and the success path are both covered.

use std::path::Path;
use std::sync::Arc;

use ftn_application::Workspace;
use ftn_domain::ids::EvidenceId;
use ftn_transcription::{
    CancelToken, EngineAdapter, EngineError, EngineProbe, ProgressFn, RawSegment,
    TranscriptionOutput, TranscriptionRequest,
};

struct NoopEngine;
impl EngineAdapter for NoopEngine {
    fn id(&self) -> &'static str {
        "noop"
    }
    fn probe(&self) -> EngineProbe {
        EngineProbe::default()
    }
    fn transcribe(
        &self,
        _r: &TranscriptionRequest,
        _c: &CancelToken,
        _p: ProgressFn<'_>,
    ) -> Result<TranscriptionOutput, EngineError> {
        Err(EngineError::Cancelled)
    }
}

fn fake_output() -> TranscriptionOutput {
    TranscriptionOutput {
        segments: vec![
            RawSegment {
                start_ms: 0,
                end_ms: 1000,
                text: "bir".to_string(),
                confidence: Some(0.5),
            },
            RawSegment {
                start_ms: 1000,
                end_ms: 2000,
                text: "iki".to_string(),
                confidence: None,
            },
        ],
        engine_version: Some("fake 1".to_string()),
        model_id: Some("fake-model".to_string()),
        settings: serde_json::json!({ "language": "tr" }),
    }
}

fn workspace(tmp: &Path) -> Workspace {
    Workspace::open(tmp.join("data"), Arc::new(NoopEngine)).unwrap()
}

#[test]
fn complete_transcription_persists_segments_and_marks_job_succeeded() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    let case = ws.create_case("Tamamla").unwrap();
    let src = tmp.path().join("kanit.bin");
    std::fs::write(&src, b"bytes").unwrap();
    let evidence = ws.import_evidence(&case.id, &src).unwrap();

    let plan = ws
        .begin_transcription(&evidence.id, Path::new("m.bin"), false)
        .unwrap();
    let job_id = plan.job.id.clone();

    let outcome = ws.complete_transcription(&plan, fake_output()).unwrap();
    assert_eq!(outcome.segments.len(), 2);

    let job = ws.get_job(&job_id).unwrap().unwrap();
    assert_eq!(job.status.as_str(), "SUCCEEDED");
    assert_eq!(ws.list_segments(&outcome.transcript.id).unwrap().len(), 2);
    assert!(ws
        .list_audit(&case.id)
        .unwrap()
        .iter()
        .any(|e| e.action.as_str() == "TRANSCRIBE_SUCCEEDED"));
}

#[test]
fn fail_transcription_records_cancelled_and_writes_no_transcript() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    let case = ws.create_case("İptal").unwrap();
    let src = tmp.path().join("kanit.bin");
    std::fs::write(&src, b"bytes").unwrap();
    let evidence = ws.import_evidence(&case.id, &src).unwrap();

    let plan = ws
        .begin_transcription(&evidence.id, Path::new("m.bin"), false)
        .unwrap();
    let job_id = plan.job.id.clone();
    assert_eq!(plan.job.status.as_str(), "RUNNING");

    ws.fail_transcription(&plan, "CANCELLED", "kullanıcı iptal etti")
        .unwrap();

    let job = ws.get_job(&job_id).unwrap().unwrap();
    assert_eq!(job.status.as_str(), "CANCELLED");
    assert_eq!(job.error_code.as_deref(), Some("CANCELLED"));
    assert!(ws.transcript_for_evidence(&evidence.id).unwrap().is_none());
    assert!(ws
        .list_audit(&case.id)
        .unwrap()
        .iter()
        .any(|e| e.action.as_str() == "TRANSCRIBE_CANCELLED"));
}

#[test]
fn begin_is_rejected_when_a_transcript_already_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    let case = ws.create_case("Tekrar").unwrap();
    let src = tmp.path().join("kanit.bin");
    std::fs::write(&src, b"bytes").unwrap();
    let evidence = ws.import_evidence(&case.id, &src).unwrap();

    let plan = ws
        .begin_transcription(&evidence.id, Path::new("m.bin"), false)
        .unwrap();
    ws.complete_transcription(&plan, fake_output()).unwrap();

    let second = ws
        .begin_transcription(&evidence.id, Path::new("m.bin"), false)
        .unwrap_err();
    assert_eq!(second.code(), "TRANSCRIPT_EXISTS");
}

#[test]
fn unknown_evidence_id_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = workspace(tmp.path());
    let err = ws
        .begin_transcription(&EvidenceId::new(), Path::new("m.bin"), false)
        .unwrap_err();
    assert_eq!(err.code(), "EVIDENCE_NOT_FOUND");
}
