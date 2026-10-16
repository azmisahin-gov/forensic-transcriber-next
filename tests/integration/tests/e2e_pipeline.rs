//! End-to-end workflow test:
//!
//! create case → import audio → verify hash → transcribe → edit segment →
//! check audit → export TXT/SRT/JSON → reopen → verify integrity.
//!
//! The ASR engine is replaced with an in-memory fake so the test is
//! deterministic. A real 16 kHz mono WAV is generated with FFmpeg when it is
//! available; if FFmpeg is missing the test is skipped and says so.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ftn_application::Workspace;
use ftn_domain::evidence::IntegrityStatus;
use ftn_domain::export_record::ExportFormat;
use ftn_media::FfmpegPaths;
use ftn_transcription::{
    CancelToken, EngineAdapter, EngineError, EngineProbe, ProgressFn, RawSegment,
    TranscriptionOutput, TranscriptionRequest,
};

/// A deterministic engine used to exercise the pipeline without whisper.cpp.
struct FakeEngine {
    segments: Vec<RawSegment>,
}

impl EngineAdapter for FakeEngine {
    fn id(&self) -> &'static str {
        "fake-engine"
    }

    fn probe(&self) -> EngineProbe {
        EngineProbe {
            available: true,
            version: Some("fake 1.0".to_string()),
            devices: Vec::new(),
            note: None,
        }
    }

    fn transcribe(
        &self,
        _request: &TranscriptionRequest,
        _cancel: &CancelToken,
        progress: ProgressFn<'_>,
    ) -> Result<TranscriptionOutput, EngineError> {
        progress(Some(0.5));
        progress(Some(1.0));
        Ok(TranscriptionOutput {
            segments: self.segments.clone(),
            engine_version: Some("fake 1.0".to_string()),
            model_id: Some("fake-model".to_string()),
            settings: serde_json::json!({ "language": "tr" }),
        })
    }
}

/// An engine that is never available, to prove the honest failure path.
struct MissingEngine;

impl EngineAdapter for MissingEngine {
    fn id(&self) -> &'static str {
        "missing-engine"
    }
    fn probe(&self) -> EngineProbe {
        EngineProbe {
            available: false,
            version: None,
            devices: Vec::new(),
            note: Some("not installed".to_string()),
        }
    }
    fn transcribe(
        &self,
        _request: &TranscriptionRequest,
        _cancel: &CancelToken,
        _progress: ProgressFn<'_>,
    ) -> Result<TranscriptionOutput, EngineError> {
        Err(EngineError::EngineNotFound("whisper-cli".to_string()))
    }
}

fn fake_segments() -> Vec<RawSegment> {
    vec![
        RawSegment {
            start_ms: 0,
            end_ms: 1200,
            text: "Merhaba, kayıt başlıyor.".to_string(),
            confidence: Some(0.9),
        },
        RawSegment {
            start_ms: 1200,
            end_ms: 2600,
            text: "İkinci cümle burada.".to_string(),
            confidence: Some(0.8),
        },
        // Invalid timing: dropped by the pipeline, counted, never persisted.
        RawSegment {
            start_ms: 2600,
            end_ms: 2600,
            text: "bozuk".to_string(),
            confidence: None,
        },
    ]
}

/// Generate a one-second 16 kHz mono WAV using FFmpeg. Returns the path, or
/// `None` when FFmpeg is unavailable (the caller then skips the test).
fn generate_wav(dir: &Path) -> Option<(PathBuf, FfmpegPaths)> {
    let ffmpeg = FfmpegPaths::resolve(None, None).ok()?;
    let out = dir.join("ornek-kayit.wav");
    let status = Command::new(&ffmpeg.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=16000:cl=mono",
            "-t",
            "1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&out)
        .status()
        .ok()?;
    if !status.success() || !out.is_file() {
        return None;
    }
    Some((out, ffmpeg))
}

fn new_engine(segments: Vec<RawSegment>) -> Arc<dyn EngineAdapter> {
    Arc::new(FakeEngine { segments })
}

fn cancel() -> CancelToken {
    Arc::new(AtomicBool::new(false))
}

#[test]
fn full_workflow_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let Some((wav, media)) = generate_wav(tmp.path()) else {
        eprintln!("SKIP full_workflow_round_trip: ffmpeg not available in this environment");
        return;
    };

    let data_dir = tmp.path().join("data");
    let mut ws = Workspace::open(&data_dir, new_engine(fake_segments()))
        .unwrap()
        .with_media(Some(media));

    // 1. Create case, 2. list it.
    let case = ws.create_case("Örnek Vaka").unwrap();
    assert_eq!(ws.list_cases().unwrap().len(), 1);

    // 3-6. Import evidence, copy + verify.
    let evidence = ws.import_evidence(&case.id, &wav).unwrap();
    assert!(evidence.is_integrity_verified());
    assert_eq!(evidence.source_sha256, evidence.stored_sha256);
    assert_eq!(evidence.metadata.sample_rate, Some(16000));
    assert_eq!(evidence.metadata.channels, Some(1));
    assert!(Path::new(&evidence.stored_path).is_file());

    // Re-verify integrity on demand.
    assert_eq!(
        ws.verify_evidence_integrity(&evidence.id).unwrap(),
        IntegrityStatus::Verified
    );

    // 7-11. Transcribe with the injected engine.
    let outcome = ws
        .start_transcription(
            &evidence.id,
            Path::new("model.bin"),
            false,
            cancel(),
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(
        outcome.segments.len(),
        2,
        "one invalid segment must be dropped"
    );
    assert_eq!(outcome.skipped_segments, 1);
    assert_eq!(outcome.job.status.as_str(), "SUCCEEDED");

    let transcript_id = outcome.transcript.id.clone();

    // 12-13. Edit a segment; original text preserved, audit recorded.
    let first = outcome.segments[0].clone();
    let updated = ws
        .edit_segment_text(&first.id, "Merhaba, kayıt başladı.")
        .unwrap();
    assert_eq!(updated.text, "Merhaba, kayıt başladı.");
    assert_eq!(updated.original_text, "Merhaba, kayıt başlıyor.");
    assert_eq!(updated.status.as_str(), "EDITED");

    let audit = ws.list_audit(&case.id).unwrap();
    assert!(audit
        .iter()
        .any(|e| e.action.as_str() == "SEGMENT_TEXT_EDITED"));
    assert!(audit
        .iter()
        .any(|e| e.action.as_str() == "EVIDENCE_IMPORTED"));
    assert!(audit
        .iter()
        .any(|e| e.action.as_str() == "TRANSCRIBE_SUCCEEDED"));

    // 14. Export TXT, SRT, JSON.
    for fmt in [ExportFormat::Txt, ExportFormat::Srt, ExportFormat::Json] {
        let record = ws.export_transcript(&case.id, &transcript_id, fmt).unwrap();
        let path = Path::new(&record.path);
        assert!(path.is_file(), "export missing: {}", record.path);
        assert!(record.sha256.is_some());
        let content = std::fs::read_to_string(path).unwrap();
        assert!(!content.is_empty());
        if fmt == ExportFormat::Json {
            let v: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert_eq!(v["schema_version"], "1.0");
            assert_eq!(v["segments"][0]["text"], "Merhaba, kayıt başladı.");
        }
    }

    // 15-16. Reopen and verify everything survives.
    drop(ws);
    let ws2 = Workspace::open(&data_dir, new_engine(fake_segments())).unwrap();
    let reopened = ws2.get_case(&case.id).unwrap();
    assert_eq!(reopened.title, "Örnek Vaka");
    let evidence2 = ws2.list_evidence(&case.id).unwrap();
    assert_eq!(evidence2.len(), 1);
    assert_eq!(
        ws2.verify_evidence_integrity(&evidence2[0].id).unwrap(),
        IntegrityStatus::Verified
    );
    let segments = ws2.list_segments(&transcript_id).unwrap();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].text, "Merhaba, kayıt başladı.");
    assert_eq!(ws2.list_exports(&case.id).unwrap().len(), 3);
}

#[test]
fn missing_engine_fails_honestly_without_writing_a_transcript() {
    let tmp = tempfile::tempdir().unwrap();
    let Some((wav, media)) = generate_wav(tmp.path()) else {
        eprintln!("SKIP missing_engine_fails_honestly: ffmpeg not available");
        return;
    };
    let data_dir = tmp.path().join("data");
    let mut ws = Workspace::open(&data_dir, Arc::new(MissingEngine))
        .unwrap()
        .with_media(Some(media));

    let case = ws.create_case("Motor Yok").unwrap();
    let evidence = ws.import_evidence(&case.id, &wav).unwrap();

    let err = ws
        .start_transcription(
            &evidence.id,
            Path::new("model.bin"),
            false,
            cancel(),
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(err.code(), "ENGINE_NOT_FOUND");

    // No transcript must exist, and the failed job must be recorded.
    assert!(ws.transcript_for_evidence(&evidence.id).unwrap().is_none());
    let jobs = ws.store().list_jobs(&evidence.id).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status.as_str(), "FAILED");
    assert_eq!(jobs[0].error_code.as_deref(), Some("ENGINE_NOT_FOUND"));
    assert!(ws
        .list_audit(&case.id)
        .unwrap()
        .iter()
        .any(|e| e.action.as_str() == "TRANSCRIBE_FAILED"));
}

#[test]
fn transcript_is_not_overwritten_by_a_second_run() {
    let tmp = tempfile::tempdir().unwrap();
    let Some((wav, media)) = generate_wav(tmp.path()) else {
        eprintln!("SKIP transcript_is_not_overwritten: ffmpeg not available");
        return;
    };
    let data_dir = tmp.path().join("data");
    let mut ws = Workspace::open(&data_dir, new_engine(fake_segments()))
        .unwrap()
        .with_media(Some(media));

    let case = ws.create_case("Tekrar").unwrap();
    let evidence = ws.import_evidence(&case.id, &wav).unwrap();
    ws.start_transcription(
        &evidence.id,
        Path::new("m.bin"),
        false,
        cancel(),
        &mut |_| {},
    )
    .unwrap();

    let second = ws
        .start_transcription(
            &evidence.id,
            Path::new("m.bin"),
            false,
            cancel(),
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(second.code(), "TRANSCRIPT_EXISTS");
}

#[test]
fn duplicate_import_never_overwrites_the_first_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let Some((wav, media)) = generate_wav(tmp.path()) else {
        eprintln!("SKIP duplicate_import: ffmpeg not available");
        return;
    };
    let data_dir = tmp.path().join("data");
    let ws = Workspace::open(&data_dir, new_engine(fake_segments()))
        .unwrap()
        .with_media(Some(media));
    let case = ws.create_case("Çift").unwrap();
    let a = ws.import_evidence(&case.id, &wav).unwrap();
    let b = ws.import_evidence(&case.id, &wav).unwrap();
    assert_ne!(a.stored_name, b.stored_name);
    assert_ne!(a.stored_path, b.stored_path);
    assert!(Path::new(&a.stored_path).is_file());
    assert!(Path::new(&b.stored_path).is_file());
}
