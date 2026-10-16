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
    CaseDto, EngineProbeDto, EvidenceDto, ExportRecordDto, JobDto, ModelDto, SegmentDto,
    StartupStatusDto, TranscriptDto, TranscriptionOutcomeDto,
};
use crate::error::CommandError;
use crate::runtime;
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeProgressPayload {
    stage: String,
    downloaded: u64,
    total: u64,
}

/// Map a poisoned lock into a safe, stable error.
fn lock_err<T>(_: std::sync::PoisonError<T>) -> CommandError {
    CommandError::new(
        "LOCK_POISONED",
        "İç durum kilidi bozuldu; uygulamayı yeniden başlatın.",
    )
}

fn build_status(state: &AppState) -> StartupStatusDto {
    let probe = state.engine_adapter().probe();
    let models_dir = runtime::models_dir(&state.data_dir_path());
    let installed: Vec<std::path::PathBuf> = std::fs::read_dir(&models_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
                .collect()
        })
        .unwrap_or_default();
    let models = runtime::MODELS
        .iter()
        .map(|spec| ModelDto {
            id: spec.id.to_string(),
            label: spec.label.to_string(),
            size_bytes: spec.size_bytes,
            installed: installed
                .iter()
                .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(spec.file)),
        })
        .collect();

    StartupStatusDto {
        ok: state.is_ready(),
        error: state.startup_error(),
        data_dir: state.data_dir(),
        engine_available: probe.available,
        engine_version: probe.version,
        engine_note: probe.note,
        engine_path: state.engine_path().map(|p| p.display().to_string()),
        engine_version_label: runtime::ENGINE_VERSION.to_string(),
        model_available: state.model_path().is_some(),
        model_path: state.model_path().map(|p| p.display().to_string()),
        models_dir: models_dir.display().to_string(),
        models,
    }
}

/// Report whether the workspace opened, what runtime is present, and which
/// models can be downloaded.
#[tauri::command]
pub fn startup_status(state: State<'_, AppState>) -> StartupStatusDto {
    build_status(&state)
}

/// Download and install the whisper.cpp engine on first use.
#[tauri::command]
pub async fn prepare_engine(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<StartupStatusDto, CommandError> {
    let data_dir = state.data_dir_path();
    let cancel_map = state.prepare_cancel();
    let cancel = cancel_token();
    *cancel_map.lock().map_err(lock_err)? = Some(cancel.clone());

    let app_task = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        runtime::install_engine(&data_dir, &cancel, |stage, downloaded, total| {
            let _ = app_task.emit(
                "runtime:progress",
                RuntimeProgressPayload {
                    stage: stage.to_string(),
                    downloaded,
                    total,
                },
            );
        })
    })
    .await
    .map_err(|e| CommandError::new("JOIN_ERROR", e.to_string()))?;

    *cancel_map.lock().map_err(lock_err)? = None;
    let binary = result.map_err(|e| CommandError::new("RUNTIME_ERROR", e.to_string()))?;
    state.set_engine_path(binary);
    Ok(build_status(&state))
}

/// Download a model by id.
#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<StartupStatusDto, CommandError> {
    let data_dir = state.data_dir_path();
    let cancel_map = state.prepare_cancel();
    let cancel = cancel_token();
    *cancel_map.lock().map_err(lock_err)? = Some(cancel.clone());

    let app_task = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        runtime::install_model(&data_dir, &model_id, &cancel, |stage, downloaded, total| {
            let _ = app_task.emit(
                "runtime:progress",
                RuntimeProgressPayload {
                    stage: stage.to_string(),
                    downloaded,
                    total,
                },
            );
        })
    })
    .await
    .map_err(|e| CommandError::new("JOIN_ERROR", e.to_string()))?;

    *cancel_map.lock().map_err(lock_err)? = None;
    let path = result.map_err(|e| CommandError::new("RUNTIME_ERROR", e.to_string()))?;
    state.set_model_path(path);
    Ok(build_status(&state))
}

/// Cancel an in-progress engine or model download.
#[tauri::command]
pub fn cancel_preparation(state: State<'_, AppState>) -> Result<bool, CommandError> {
    let cancel_map = state.prepare_cancel();
    let guard = cancel_map.lock().map_err(lock_err)?;
    match guard.as_ref() {
        Some(token) => {
            token.store(true, Ordering::Relaxed);
            Ok(true)
        }
        None => Ok(false),
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
    model_path: Option<String>,
    use_gpu: bool,
    language: Option<String>,
) -> Result<TranscriptionOutcomeDto, CommandError> {
    let workspace = state.workspace()?;
    let jobs = state.jobs();
    let evidence_id = EvidenceId::from_string(evidence_id);

    let model = match model_path {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => state.model_path().ok_or_else(|| {
            CommandError::new(
                "MODEL_NOT_FOUND",
                "Transkripsiyon modeli bulunamadı. Hazırlık ekranından indirin.",
            )
        })?,
    };

    // Short phase: validate, create the job, build the plan.
    let plan = {
        let mut ws = workspace.lock().map_err(lock_err)?;
        let mut plan = ws.begin_transcription(&evidence_id, &model, use_gpu)?;
        if let Some(lang) = language {
            if !lang.trim().is_empty() {
                plan.request.language = lang;
            }
        }
        plan
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
