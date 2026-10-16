//! The workspace: the single place where user operations are orchestrated.
//! The UI has no business rules of its own; it calls these methods through the
//! Tauri command surface.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ftn_domain::audit::{AuditAction, AuditEvent};
use ftn_domain::case::Case;
use ftn_domain::evidence::{
    DerivedKind, DerivedMediaAsset, EvidenceFile, IntegrityStatus, MediaMetadata,
};
use ftn_domain::export_record::{ExportFormat, ExportRecord};
use ftn_domain::ids::{CaseId, DerivedAssetId, EvidenceId, JobId, SegmentId, TranscriptId};
use ftn_domain::job::TranscriptionJob;
use ftn_domain::revision::{RevisionState, TranscriptRevision};
use ftn_domain::time::Timestamp;
use ftn_domain::transcript::{SpeakerLabel, Transcript, TranscriptSegment};
use ftn_domain::APP_VERSION;
use ftn_export::{self as export_engine};
use ftn_integrity as integrity;
use ftn_media::{probe_media, FfmpegPaths};
use ftn_storage::{SqliteStore, StorageError};
use ftn_transcription::{
    new_cancel_token, CancelToken, EngineAdapter, TranscriptionOutput, TranscriptionRequest,
};

use crate::error::AppError;
use crate::paths::WorkspacePaths;

/// The result of a transcription attempt.
#[derive(Debug, Clone)]
pub struct TranscriptionOutcome {
    pub transcript: Transcript,
    pub segments: Vec<TranscriptSegment>,
    pub job: TranscriptionJob,
    /// Number of raw segments dropped because their timing was invalid.
    pub skipped_segments: usize,
}

/// The application workspace.
pub struct Workspace {
    store: SqliteStore,
    paths: WorkspacePaths,
    engine: Arc<dyn EngineAdapter>,
    media: Option<FfmpegPaths>,
    app_version: String,
}

impl Workspace {
    /// Open (creating if needed) a workspace whose database lives at
    /// `<data_dir>/case.sqlite`.
    pub fn open(
        data_dir: impl Into<PathBuf>,
        engine: Arc<dyn EngineAdapter>,
    ) -> Result<Self, AppError> {
        let paths = WorkspacePaths::new(data_dir);
        paths.ensure_root()?;
        let store = SqliteStore::open(paths.database_path())?;
        Ok(Self {
            store,
            paths,
            engine,
            media: None,
            app_version: APP_VERSION.to_string(),
        })
    }

    /// Open an in-memory workspace (used by tests).
    pub fn in_memory(engine: Arc<dyn EngineAdapter>) -> Result<Self, AppError> {
        let store = SqliteStore::in_memory()?;
        Ok(Self {
            store,
            paths: WorkspacePaths::new(std::env::temp_dir().join("ftn-in-memory")),
            engine,
            media: None,
            app_version: APP_VERSION.to_string(),
        })
    }

    pub fn with_media(mut self, media: Option<FfmpegPaths>) -> Self {
        self.media = media;
        self
    }

    pub fn store(&self) -> &SqliteStore {
        &self.store
    }

    pub fn paths(&self) -> &WorkspacePaths {
        &self.paths
    }

    pub fn app_version(&self) -> &str {
        &self.app_version
    }

    // ---- cases -----------------------------------------------------------

    pub fn create_case(&self, title: &str) -> Result<Case, AppError> {
        let case = Case::new(title)?;
        self.store.create_case(&case)?;
        self.paths.ensure_case_dirs(&case.id)?;
        self.record(
            AuditEvent::new(case.id.clone(), AuditAction::CaseCreated, &self.app_version)
                .with_change(None, Some(serde_json::json!({ "title": case.title }))),
        );
        Ok(case)
    }

    pub fn list_cases(&self) -> Result<Vec<Case>, AppError> {
        Ok(self.store.list_cases()?)
    }

    pub fn get_case(&self, id: &CaseId) -> Result<Case, AppError> {
        self.store
            .get_case(id)
            .map_err(|e| not_found(e, AppError::CaseNotFound(id.to_string())))
    }

    pub fn rename_case(&self, id: &CaseId, title: &str) -> Result<Case, AppError> {
        let mut case = self.get_case(id)?;
        let before = serde_json::json!({ "title": case.title });
        case.rename(title)?;
        self.store.update_case(&case)?;
        self.record(
            AuditEvent::new(case.id.clone(), AuditAction::CaseRenamed, &self.app_version)
                .with_change(
                    Some(before),
                    Some(serde_json::json!({ "title": case.title })),
                ),
        );
        Ok(case)
    }

    // ---- evidence --------------------------------------------------------

    /// Import an audio file: hash it, copy it into the managed store, verify the
    /// copy, probe its metadata, persist the record and audit the action. The
    /// user's original file is never modified.
    pub fn import_evidence(
        &self,
        case_id: &CaseId,
        source: &Path,
    ) -> Result<EvidenceFile, AppError> {
        let _case = self.get_case(case_id)?;
        if !source.is_file() {
            return Err(AppError::Media(ftn_media::MediaError::InputMissing(
                source.to_path_buf(),
            )));
        }
        self.paths.ensure_case_dirs(case_id)?;

        let original_name = source.file_name().and_then(|s| s.to_str()).ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "source has no file name",
            ))
        })?;

        let dest_dir = self.paths.evidence_original_dir(case_id);
        let stored_name = integrity::unique_file_name(&dest_dir, original_name)?;
        let copy = integrity::copy_and_verify(source, &dest_dir, &stored_name, |_| {})?;

        let metadata = match &self.media {
            Some(m) => probe_media(&m.ffprobe, &copy.dest_path)
                .map(|p| p.metadata)
                .unwrap_or_default(),
            None => MediaMetadata::default(),
        };

        let evidence = EvidenceFile {
            id: EvidenceId::new(),
            case_id: case_id.clone(),
            original_name: original_name.to_string(),
            stored_name,
            original_path: source.display().to_string(),
            stored_path: copy.dest_path.display().to_string(),
            size_bytes: copy.bytes,
            metadata,
            source_sha256: copy.source_sha256.clone(),
            stored_sha256: copy.copied_sha256.clone(),
            integrity: IntegrityStatus::Verified,
            imported_at: Timestamp::now(),
        };

        self.store.insert_evidence_for_case(case_id, &evidence)?;
        self.record(
            AuditEvent::new(
                case_id.clone(),
                AuditAction::EvidenceImported,
                &self.app_version,
            )
            .with_evidence(evidence.id.clone())
            .with_change(
                None,
                Some(serde_json::json!({
                    "original_name": evidence.original_name,
                    "sha256": evidence.source_sha256.as_str(),
                    "size_bytes": evidence.size_bytes,
                })),
            ),
        );
        Ok(evidence)
    }

    pub fn list_evidence(&self, case_id: &CaseId) -> Result<Vec<EvidenceFile>, AppError> {
        Ok(self.store.list_evidence(case_id)?)
    }

    pub fn get_evidence(&self, id: &EvidenceId) -> Result<EvidenceFile, AppError> {
        self.store
            .get_evidence(id)
            .map_err(|e| not_found(e, AppError::EvidenceNotFound(id.to_string())))
    }

    /// Re-hash the stored copy and compare with the digest recorded at import.
    pub fn verify_evidence_integrity(
        &self,
        evidence_id: &EvidenceId,
    ) -> Result<IntegrityStatus, AppError> {
        let mut evidence = self.get_evidence(evidence_id)?;
        let stored_path = PathBuf::from(&evidence.stored_path);

        let (status, new_digest) = if !stored_path.is_file() {
            (IntegrityStatus::Missing, evidence.stored_sha256.clone())
        } else {
            match integrity::hash_file(&stored_path) {
                Ok(actual) => {
                    if actual == evidence.source_sha256 {
                        (IntegrityStatus::Verified, actual)
                    } else {
                        (IntegrityStatus::Mismatch, actual)
                    }
                }
                Err(_) => (IntegrityStatus::Missing, evidence.stored_sha256.clone()),
            }
        };

        self.store
            .set_evidence_integrity(evidence_id, status, &new_digest)?;
        evidence.integrity = status;
        evidence.stored_sha256 = new_digest.clone();

        self.record(
            AuditEvent::new(
                evidence.case_id.clone(),
                AuditAction::EvidenceIntegrityChecked,
                &self.app_version,
            )
            .with_evidence(evidence_id.clone())
            .with_change(
                None,
                Some(serde_json::json!({
                    "status": format!("{status:?}"),
                    "recomputed_sha256": new_digest.as_str(),
                })),
            ),
        );
        Ok(status)
    }

    // ---- transcription ---------------------------------------------------

    /// Prepare a transcription without running the engine (short DB phase).
    ///
    /// Validates the input, records the start, creates a queued→running job and
    /// returns a [`TranscriptionPlan`] the caller executes **outside** any lock.
    /// Persisting the result happens later in [`Self::complete_transcription`],
    /// so the workspace mutex is never held for the whole engine run.
    pub fn begin_transcription(
        &mut self,
        evidence_id: &EvidenceId,
        model_path: &Path,
        use_gpu: bool,
    ) -> Result<TranscriptionPlan, AppError> {
        let evidence = self.get_evidence(evidence_id)?;
        let case_id = evidence.case_id.clone();

        if self
            .store
            .get_transcript_by_evidence(evidence_id)?
            .is_some()
        {
            return Err(AppError::TranscriptExists);
        }

        self.record(
            AuditEvent::new(
                case_id.clone(),
                AuditAction::TranscribeStarted,
                &self.app_version,
            )
            .with_evidence(evidence_id.clone()),
        );

        let working_path = self.paths.derived_wav_path(&case_id, evidence_id);
        let needs_transcode = !(evidence
            .metadata
            .format
            .as_deref()
            .map(|f| f.contains("wav"))
            .unwrap_or(false)
            && evidence.metadata.sample_rate == Some(16_000)
            && evidence.metadata.channels == Some(1));

        let input_for_engine = if needs_transcode {
            working_path.clone()
        } else {
            PathBuf::from(&evidence.stored_path)
        };

        let request = TranscriptionRequest {
            job_evidence_id: evidence_id.clone(),
            input: input_for_engine,
            model: model_path.to_path_buf(),
            language: "tr".to_string(),
            threads: None,
            use_gpu,
        };

        let mut job = TranscriptionJob::queued(evidence_id.clone());
        job.start();
        job.engine = Some(ftn_domain::job::EngineInfo {
            engine: self.engine.id().to_string(),
            engine_version: None,
            model_id: None,
            model_sha256: None,
            settings: serde_json::json!({ "language": "tr", "use_gpu": use_gpu }),
        });
        self.store.insert_job(&job)?;

        let transcript = Transcript::new(evidence_id.clone(), "tr");

        Ok(TranscriptionPlan {
            case_id,
            evidence,
            transcript,
            job,
            engine: self.engine.clone(),
            media: self.media.clone(),
            working_path,
            needs_transcode,
            request,
        })
    }

    /// Persist the result of a completed engine run (short DB phase).
    pub fn complete_transcription(
        &mut self,
        plan: &TranscriptionPlan,
        output: TranscriptionOutput,
    ) -> Result<TranscriptionOutcome, AppError> {
        if plan.needs_transcode {
            self.paths.ensure_case_dirs(&plan.case_id)?;
            if let Ok(hash) = integrity::hash_file(&plan.working_path) {
                let asset = DerivedMediaAsset {
                    id: DerivedAssetId::new(),
                    evidence_id: plan.evidence.id.clone(),
                    path: plan.working_path.display().to_string(),
                    kind: DerivedKind::TranscriptionInput,
                    format: "wav".to_string(),
                    sample_rate: 16000,
                    channels: 1,
                    sha256: hash,
                    generator: "ffmpeg -vn -ac 1 -ar 16000 -c:a pcm_s16le".to_string(),
                    created_at: Timestamp::now(),
                };
                let _ = self.store.insert_derived_asset(&asset);
            }
        }

        let mut transcript = plan.transcript.clone();
        transcript.engine = output
            .engine_version
            .clone()
            .or_else(|| Some(plan.engine.id().to_string()));
        transcript.model_id = output.model_id.clone();

        let mut segments = Vec::with_capacity(output.segments.len());
        let mut skipped = 0usize;
        for (ordinal, raw) in output.segments.iter().enumerate() {
            match TranscriptSegment::automatic(
                transcript.id.clone(),
                ordinal as u32,
                raw.start_ms,
                raw.end_ms,
                SpeakerLabel::default_speaker(),
                raw.text.clone(),
            ) {
                Ok(mut seg) => {
                    seg.confidence = raw.confidence;
                    segments.push(seg);
                }
                Err(_) => skipped += 1,
            }
        }

        self.store.create_transcript(&plan.case_id, &transcript)?;
        self.store.replace_segments(&transcript.id, &segments)?;
        let revision = TranscriptRevision::new(
            transcript.id.clone(),
            Some(plan.job.id.clone()),
            RevisionState::Machine,
            segments.clone(),
        );
        self.store.append_revision(&revision)?;

        let mut job = plan.job.clone();
        job.succeed(transcript.id.clone());
        if let Some(engine) = job.engine.as_mut() {
            engine.engine_version = output.engine_version.clone();
            engine.model_id = output.model_id.clone();
        }
        self.store.update_job(&job)?;

        self.record(
            AuditEvent::new(
                plan.case_id.clone(),
                AuditAction::TranscribeSucceeded,
                &self.app_version,
            )
            .with_evidence(plan.evidence.id.clone())
            .with_transcript(transcript.id.clone())
            .with_change(
                None,
                Some(serde_json::json!({
                    "segments": segments.len(),
                    "skipped": skipped,
                    "model_id": output.model_id,
                })),
            ),
        );

        Ok(TranscriptionOutcome {
            transcript,
            segments,
            job,
            skipped_segments: skipped,
        })
    }

    /// Record a failed or cancelled run (short DB phase). No transcript is written.
    pub fn fail_transcription(
        &mut self,
        plan: &TranscriptionPlan,
        code: &str,
        message: &str,
    ) -> Result<(), AppError> {
        let mut job = plan.job.clone();
        if code == "CANCELLED" {
            job.cancel_with(code, message);
        } else {
            job.fail(code, message);
        }
        self.store.update_job(&job)?;
        let action = if code == "CANCELLED" {
            AuditAction::TranscribeCancelled
        } else {
            AuditAction::TranscribeFailed
        };
        self.record(
            AuditEvent::new(plan.case_id.clone(), action, &self.app_version)
                .with_evidence(plan.evidence.id.clone())
                .with_change(None, Some(serde_json::json!({ "code": code }))),
        );
        Ok(())
    }

    /// Convenience: begin + execute + complete in one call. Used by tests and by
    /// callers that do not need to release the lock between phases.
    pub fn start_transcription(
        &mut self,
        evidence_id: &EvidenceId,
        model_path: &Path,
        use_gpu: bool,
        cancel: CancelToken,
        progress: &mut dyn FnMut(Option<f32>),
    ) -> Result<TranscriptionOutcome, AppError> {
        let plan = self.begin_transcription(evidence_id, model_path, use_gpu)?;
        match execute_transcription(&plan, &cancel, progress) {
            Ok(output) => self.complete_transcription(&plan, output),
            Err(err) => {
                let _ = self.fail_transcription(&plan, err.code(), &err.to_string());
                Err(err)
            }
        }
    }

    /// Load a single job by id (used by the UI to poll status).
    pub fn get_job(&self, job_id: &JobId) -> Result<Option<TranscriptionJob>, AppError> {
        Ok(self.store.get_job(job_id)?)
    }

    pub fn transcript_for_evidence(
        &self,
        evidence_id: &EvidenceId,
    ) -> Result<Option<Transcript>, AppError> {
        Ok(self.store.get_transcript_by_evidence(evidence_id)?)
    }

    pub fn list_segments(
        &self,
        transcript_id: &TranscriptId,
    ) -> Result<Vec<TranscriptSegment>, AppError> {
        Ok(self.store.list_segments(transcript_id)?)
    }

    // ---- review / edit ---------------------------------------------------

    /// Edit a segment's text, preserving the automatic text. Returns the updated
    /// segment.
    pub fn edit_segment_text(
        &mut self,
        segment_id: &SegmentId,
        new_text: &str,
    ) -> Result<TranscriptSegment, AppError> {
        let mut segment = self
            .store
            .get_segment(segment_id)
            .map_err(|e| not_found(e, AppError::SegmentNotFound(segment_id.to_string())))?;
        let before = segment.text.clone();
        segment.edit_text(new_text)?;
        self.store.update_segment(&segment)?;

        let transcript = self.store.get_transcript(&segment.transcript_id)?;
        let evidence = self.store.get_evidence(&transcript.evidence_id)?;
        if before != segment.text {
            self.record(
                AuditEvent::new(
                    evidence.case_id,
                    AuditAction::SegmentTextEdited,
                    &self.app_version,
                )
                .with_evidence(evidence.id)
                .with_transcript(segment.transcript_id.clone())
                .with_segment(segment.id.clone())
                .with_change(
                    Some(serde_json::json!({ "text": before })),
                    Some(serde_json::json!({ "text": segment.text })),
                ),
            );
        }
        Ok(segment)
    }

    // ---- export ----------------------------------------------------------

    /// Render a transcript to the requested format and write it into the case's
    /// export directory. The write is atomic and the output is hashed.
    pub fn export_transcript(
        &self,
        case_id: &CaseId,
        transcript_id: &TranscriptId,
        format: ExportFormat,
    ) -> Result<ExportRecord, AppError> {
        let case = self.get_case(case_id)?;
        let transcript = self
            .store
            .get_transcript(transcript_id)
            .map_err(|e| not_found(e, AppError::TranscriptNotFound(transcript_id.to_string())))?;
        let evidence = self.store.get_evidence(&transcript.evidence_id)?;
        let segments = self.store.list_segments(transcript_id)?;

        let meta = export_engine::ExportMeta {
            case_id: case.id.to_string(),
            case_title: case.title.clone(),
            evidence_name: evidence.original_name.clone(),
            transcript_id: transcript.id.to_string(),
            language: transcript.language.clone(),
            engine: transcript.engine.clone(),
            model_id: transcript.model_id.clone(),
            generated_at: Timestamp::now().to_rfc3339(),
            app_version: self.app_version.clone(),
        };
        let rendered = export_engine::render(format, &meta, &segments)?;

        let file_name = export_engine::safe_export_file_name(&case.title, format)?;
        let exports_dir = self.paths.exports_dir(case_id);
        std::fs::create_dir_all(&exports_dir)?;
        let final_path = exports_dir.join(&file_name);
        let tmp_path = exports_dir.join(format!(".{file_name}.part"));
        std::fs::write(&tmp_path, rendered.as_bytes())?;
        std::fs::rename(&tmp_path, &final_path)?;

        let digest = integrity::hash_file(&final_path)?;
        let mut record = ExportRecord::new(
            case_id.clone(),
            format,
            final_path.display().to_string(),
            export_engine::TRANSCRIPT_SCHEMA_VERSION,
        );
        record.transcript_id = Some(transcript.id.clone());
        record.evidence_id = Some(evidence.id.clone());
        record.sha256 = Some(digest.as_str().to_string());
        self.store.insert_export(&record)?;

        self.record(
            AuditEvent::new(
                case_id.clone(),
                AuditAction::ExportCreated,
                &self.app_version,
            )
            .with_transcript(transcript.id.clone())
            .with_change(
                None,
                Some(serde_json::json!({ "format": format.as_str(), "path": record.path })),
            ),
        );
        Ok(record)
    }

    pub fn list_exports(&self, case_id: &CaseId) -> Result<Vec<ExportRecord>, AppError> {
        Ok(self.store.list_exports(case_id)?)
    }

    // ---- audit -----------------------------------------------------------

    pub fn list_audit(&self, case_id: &CaseId) -> Result<Vec<AuditEvent>, AppError> {
        Ok(self.store.list_audit(case_id)?)
    }

    fn record(&self, event: AuditEvent) {
        // Auditing must never abort the user operation; a failure is ignored but
        // still visible through the standard storage error path elsewhere.
        let _ = self.store.append_audit(&event);
    }
}

fn not_found(err: StorageError, mapped: AppError) -> AppError {
    match err {
        StorageError::NotFound { .. } => mapped,
        other => AppError::Storage(other),
    }
}

/// A prepared transcription that can be executed without the workspace lock.
///
/// It carries everything the engine and media layers need: the stored paths,
/// whether a working copy must be produced, the cloned engine adapter and the
/// already-persisted job. The database is only touched again in
/// [`Workspace::complete_transcription`] / [`Workspace::fail_transcription`].
pub struct TranscriptionPlan {
    pub case_id: CaseId,
    pub evidence: EvidenceFile,
    pub transcript: Transcript,
    pub job: TranscriptionJob,
    pub engine: Arc<dyn EngineAdapter>,
    pub media: Option<FfmpegPaths>,
    pub working_path: PathBuf,
    pub needs_transcode: bool,
    pub request: TranscriptionRequest,
}

impl TranscriptionPlan {
    /// The job id as a string, for correlating progress and cancel requests.
    pub fn job_id(&self) -> String {
        self.job.id.to_string()
    }
}

impl std::fmt::Debug for TranscriptionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriptionPlan")
            .field("case_id", &self.case_id)
            .field("evidence_id", &self.evidence.id)
            .field("job_id", &self.job.id)
            .field("needs_transcode", &self.needs_transcode)
            .finish_non_exhaustive()
    }
}

/// Produce the working audio if needed and run the engine.
///
/// This function touches no database state, so it can run on a background thread
/// without holding the workspace mutex.
pub fn execute_transcription(
    plan: &TranscriptionPlan,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Option<f32>),
) -> Result<TranscriptionOutput, AppError> {
    if plan.needs_transcode {
        let media = plan
            .media
            .as_ref()
            .ok_or_else(|| AppError::MediaToolMissing("ffmpeg".to_string()))?;
        if let Some(parent) = plan.working_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        ftn_media::ffmpeg::transcode_to_16k_mono_wav(
            &media.ffmpeg,
            Path::new(&plan.evidence.stored_path),
            &plan.working_path,
        )?;
    }
    plan.engine
        .transcribe(&plan.request, cancel, progress)
        .map_err(AppError::Engine)
}

/// Convenience: a fresh cancellation token.
pub fn cancel_token() -> CancelToken {
    new_cancel_token()
}
