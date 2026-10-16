//! The Tauri command surface. Every parameter coming from the frontend is
//! treated as untrusted input and handled by the application layer, which
//! validates it. The UI is never the security boundary.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use ftn_application::cancel_token;
use ftn_domain::export_record::ExportFormat;
use ftn_domain::ids::{CaseId, EvidenceId, JobId, SegmentId, TranscriptId};
use ftn_transcription::EngineAdapter;
use serde::Serialize;
use tauri::{Emitter, State};

use crate::dto::{
    CaseDto, EngineProbeDto, EvidenceDto, ExportRecordDto, JobDto, SegmentDto, StartupStatusDto,
    TranscriptDto, TranscriptionOutcomeDto,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Event channel names consumed by the frontend.
const EVENT_PROGRESS: &str = "transcription:progress";
const EVENT_STAGE: &str = "transcription:stage";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    job_id: String,
    fraction: Option<f32>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StagePayload {
    job_id: String,
    stage: String,
}

/// Map a poisoned lock into a safe, stable error.
fn lock_err<T>(_: std::sync::PoisonError<T>) -> CommandError {
    CommandError::new(
        "LOCK_POISONED",
        "İç durum kilidi bozuldu; uygulamayı yeniden başlatın.",
    )
}

/// Report whether the workspace opened and whether an engine is configured.
#[tauri::command]
pub fn startup_status(state: State<'_, AppState>) -> StartupStatusDto {
    let probe = state.engine_adapter().probe();
    StartupStatusDto {
        ok: state.is_ready(),
        error: state.startup_error(),
        data_dir: state.data_dir(),
        engine_available: probe.available,
        engine_version: probe.version,
        engine_note: probe.note,
    }
}

#[tauri::command]
pub fn list_cases(state: State<'_, AppState>) -> Result<Vec<CaseDto>, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    Ok(ws.list_cases()?.iter().map(CaseDto::from).collect())
}

#[tauri::command]
pub fn create_case(state: State<'_, AppState>, title: String) -> Result<CaseDto, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    let case = ws.create_case(&title)?;
    Ok(CaseDto::from(&case))
}

#[tauri::command]
pub fn get_case(state: State<'_, AppState>, case_id: String) -> Result<CaseDto, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    let case = ws.get_case(&CaseId::from_string(case_id))?;
    Ok(CaseDto::from(&case))
}

#[tauri::command]
pub fn list_evidence(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Vec<EvidenceDto>, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    Ok(ws
        .list_evidence(&CaseId::from_string(case_id))?
        .iter()
        .map(EvidenceDto::from)
        .collect())
}

#[tauri::command]
pub fn import_evidence(
    state: State<'_, AppState>,
    case_id: String,
    path: String,
) -> Result<EvidenceDto, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    let evidence = ws.import_evidence(&CaseId::from_string(case_id), Path::new(&path))?;
    Ok(EvidenceDto::from(&evidence))
}

#[tauri::command]
pub fn verify_evidence(
    state: State<'_, AppState>,
    evidence_id: String,
) -> Result<String, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    let status = ws.verify_evidence_integrity(&EvidenceId::from_string(evidence_id))?;
    Ok(format!("{status:?}").to_uppercase())
}

/// Run a transcription on a background thread.
///
/// The workspace lock is held only for the short prepare and persist phases; the
/// engine run happens without it, so listing cases and reading evidence keep
/// working while a transcription is in progress. Progress is emitted on
/// `transcription:progress` and the job's lifecycle on `transcription:stage`.
#[tauri::command]
pub async fn transcribe(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    evidence_id: String,
    model_path: String,
    use_gpu: bool,
) -> Result<TranscriptionOutcomeDto, CommandError> {
    let workspace = state.workspace()?;
    let jobs = state.jobs();
    let evidence_id = EvidenceId::from_string(evidence_id);
    let model = PathBuf::from(model_path);

    // Short phase: validate, create the job, build the plan.
    let plan = {
        let mut ws = workspace.lock().map_err(lock_err)?;
        ws.begin_transcription(&evidence_id, &model, use_gpu)?
    };
    let job_id = plan.job_id();

    let cancel = cancel_token();
    jobs.lock()
        .map_err(lock_err)?
        .insert(job_id.clone(), cancel.clone());

    let app_task = app.clone();
    let job_for_events = job_id.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let _ = app_task.emit(
            EVENT_STAGE,
            StagePayload {
                job_id: job_for_events.clone(),
                stage: "running".to_string(),
            },
        );
        let mut progress = |fraction: Option<f32>| {
            let _ = app_task.emit(
                EVENT_PROGRESS,
                ProgressPayload {
                    job_id: job_for_events.clone(),
                    fraction,
                },
            );
        };

        match ftn_application::execute_transcription(&plan, &cancel, &mut progress) {
            Ok(output) => {
                let mut ws = workspace.lock().map_err(lock_err)?;
                ws.complete_transcription(&plan, output)
                    .map_err(CommandError::from)
            }
            Err(err) => {
                let code = err.code().to_string();
                let message = err.to_string();
                let mut ws = workspace.lock().map_err(lock_err)?;
                let _ = ws.fail_transcription(&plan, &code, &message);
                Err(CommandError::from(err))
            }
        }
    })
    .await
    .map_err(|e| CommandError::new("JOIN_ERROR", e.to_string()))??;

    jobs.lock().map_err(lock_err)?.remove(&job_id);
    let _ = app.emit(
        EVENT_STAGE,
        StagePayload {
            job_id,
            stage: "finished".to_string(),
        },
    );

    Ok(TranscriptionOutcomeDto {
        transcript: TranscriptDto::from(&outcome.transcript),
        segments: outcome.segments.iter().map(SegmentDto::from).collect(),
        job: JobDto::from(&outcome.job),
        skipped_segments: outcome.skipped_segments,
    })
}

/// Request cancellation of a running job. Returns whether a matching job existed.
#[tauri::command]
pub fn cancel_transcription(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<bool, CommandError> {
    let jobs = state.jobs();
    let jobs = jobs.lock().map_err(lock_err)?;
    match jobs.get(&job_id) {
        Some(token) => {
            token.store(true, Ordering::Relaxed);
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Poll the durable status of a job.
#[tauri::command]
pub fn job_status(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<JobDto>, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    Ok(ws
        .get_job(&JobId::from_string(job_id))?
        .as_ref()
        .map(JobDto::from))
}

#[tauri::command]
pub fn transcript_for_evidence(
    state: State<'_, AppState>,
    evidence_id: String,
) -> Result<Option<TranscriptDto>, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    Ok(ws
        .transcript_for_evidence(&EvidenceId::from_string(evidence_id))?
        .as_ref()
        .map(TranscriptDto::from))
}

#[tauri::command]
pub fn list_segments(
    state: State<'_, AppState>,
    transcript_id: String,
) -> Result<Vec<SegmentDto>, CommandError> {
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    Ok(ws
        .list_segments(&TranscriptId::from_string(transcript_id))?
        .iter()
        .map(SegmentDto::from)
        .collect())
}

#[tauri::command]
pub fn edit_segment(
    state: State<'_, AppState>,
    segment_id: String,
    text: String,
) -> Result<SegmentDto, CommandError> {
    let workspace = state.workspace()?;
    let mut ws = workspace.lock().map_err(lock_err)?;
    let updated = ws.edit_segment_text(&SegmentId::from_string(segment_id), &text)?;
    Ok(SegmentDto::from(&updated))
}

#[tauri::command]
pub fn export_transcript(
    state: State<'_, AppState>,
    case_id: String,
    transcript_id: String,
    format: String,
) -> Result<ExportRecordDto, CommandError> {
    let export_format = ExportFormat::parse(&format).ok_or_else(|| {
        CommandError::new("UNKNOWN_FORMAT", format!("Bilinmeyen biçim: {format}"))
    })?;
    let workspace = state.workspace()?;
    let ws = workspace.lock().map_err(lock_err)?;
    let record = ws.export_transcript(
        &CaseId::from_string(case_id),
        &TranscriptId::from_string(transcript_id),
        export_format,
    )?;
    Ok(ExportRecordDto::from(&record))
}

#[tauri::command]
pub fn engine_probe(state: State<'_, AppState>) -> EngineProbeDto {
    let probe = state.engine_adapter().probe();
    EngineProbeDto {
        available: probe.available,
        version: probe.version,
        note: probe.note,
    }
}
