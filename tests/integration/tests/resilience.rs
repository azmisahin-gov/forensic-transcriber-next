//! Resilience tests: damaged evidence, missing files, bad paths, unicode
//! titles, and re-open behaviour. These do not need FFmpeg.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use ftn_application::Workspace;
use ftn_domain::evidence::IntegrityStatus;
use ftn_domain::export_record::ExportFormat;
use ftn_transcription::{
    CancelToken, EngineAdapter, EngineError, EngineProbe, ProgressFn, TranscriptionOutput,
    TranscriptionRequest,
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
        Err(EngineError::EngineNotFound("noop".into()))
    }
}

fn ws(dir: &Path) -> Workspace {
    Workspace::open(dir.join("data"), Arc::new(NoopEngine)).unwrap()
}

#[test]
fn importing_a_missing_file_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = ws(tmp.path());
    let case = ws.create_case("Vaka").unwrap();
    let err = ws
        .import_evidence(&case.id, Path::new("C:/no/such/file.wav"))
        .unwrap_err();
    assert_eq!(err.code(), "MEDIA_ERROR");
}

#[test]
fn tampered_stored_copy_is_reported_as_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = ws(tmp.path());
    let case = ws.create_case("Kurcalanmış").unwrap();

    // Import a plain file (no ffmpeg needed: metadata stays empty).
    let src = tmp.path().join("kanit.bin");
    std::fs::write(&src, b"original evidence bytes").unwrap();
    let evidence = ws.import_evidence(&case.id, &src).unwrap();
    assert_eq!(
        ws.verify_evidence_integrity(&evidence.id).unwrap(),
        IntegrityStatus::Verified
    );

    // Overwrite the managed copy.
    std::fs::write(&evidence.stored_path, b"tampered bytes").unwrap();
    assert_eq!(
        ws.verify_evidence_integrity(&evidence.id).unwrap(),
        IntegrityStatus::Mismatch
    );
}

#[test]
fn deleted_stored_copy_is_reported_as_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = ws(tmp.path());
    let case = ws.create_case("Silinmiş").unwrap();
    let src = tmp.path().join("kanit.bin");
    std::fs::write(&src, b"bytes").unwrap();
    let evidence = ws.import_evidence(&case.id, &src).unwrap();
    std::fs::remove_file(&evidence.stored_path).unwrap();
    assert_eq!(
        ws.verify_evidence_integrity(&evidence.id).unwrap(),
        IntegrityStatus::Missing
    );
}

#[test]
fn unicode_case_title_exports_to_a_safe_file_name() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = ws(tmp.path());
    let case = ws.create_case("İçişleri Bakanlığı / Örnek: Vaka").unwrap();
    // No transcript, so we only check the naming helper indirectly by creating a
    // transcript-free export is not possible; instead assert the title round
    // trips and the case directory exists with the dynamic id.
    assert!(ws.paths().case_dir(&case.id).is_dir());
    // The dangerous characters must not appear in a derived file name.
    use ftn_export::safe_export_file_name;
    let name = safe_export_file_name(&case.title, ExportFormat::Json).unwrap();
    assert!(!name.contains('/'));
    assert!(!name.contains('\\'));
    assert!(!name.contains(':'));
}

#[test]
fn reopening_a_workspace_preserves_case_count() {
    let tmp = tempfile::tempdir().unwrap();
    {
        let ws = ws(tmp.path());
        ws.create_case("Bir").unwrap();
        ws.create_case("İki").unwrap();
    }
    let ws2 = ws(tmp.path());
    assert_eq!(ws2.list_cases().unwrap().len(), 2);
}

#[test]
fn cancel_token_default_is_not_cancelled() {
    let token: CancelToken = Arc::new(AtomicBool::new(false));
    assert!(!token.load(std::sync::atomic::Ordering::Relaxed));
}
