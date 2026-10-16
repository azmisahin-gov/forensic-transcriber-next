//! The Tauri command surface. Every parameter coming from the frontend is
//! treated as untrusted input and handled by the application layer, which
//! validates it. The UI is never the security boundary.

use std::path::Path;

use ftn_application::cancel_token;
use ftn_domain::export_record::ExportFormat;
use ftn_domain::ids::{CaseId, EvidenceId, SegmentId, TranscriptId};
use ftn_transcription::EngineAdapter;
use tauri::State;

use crate::dto::{
    CaseDto, EngineProbeDto, EvidenceDto, ExportRecordDto, SegmentDto, TranscriptDto,
    TranscriptionOutcomeDto,
};
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command]
pub fn list_cases(state: State<'_, AppState>) -> Result<Vec<CaseDto>, CommandError> {
    let ws = state.workspace.lock()?;
    Ok(ws.list_cases()?.iter().map(CaseDto::from).collect())
}

#[tauri::command]
pub fn create_case(state: State<'_, AppState>, title: String) -> Result<CaseDto, CommandError> {
    let ws = state.workspace.lock()?;
    let case = ws.create_case(&title)?;
    Ok(CaseDto::from(&case))
}

#[tauri::command]
pub fn get_case(state: State<'_, AppState>, case_id: String) -> Result<CaseDto, CommandError> {
    let ws = state.workspace.lock()?;
    let case = ws.get_case(&CaseId::from_string(case_id))?;
    Ok(CaseDto::from(&case))
}

#[tauri::command]
pub fn list_evidence(
    state: State<'_, AppState>,
    case_id: String,
) -> Result<Vec<EvidenceDto>, CommandError> {
    let ws = state.workspace.lock()?;
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
    let ws = state.workspace.lock()?;
    let evidence = ws.import_evidence(&CaseId::from_string(case_id), Path::new(&path))?;
    Ok(EvidenceDto::from(&evidence))
}

#[tauri::command]
pub fn verify_evidence(
    state: State<'_, AppState>,
    evidence_id: String,
) -> Result<String, CommandError> {
    let ws = state.workspace.lock()?;
    let status = ws.verify_evidence_integrity(&EvidenceId::from_string(evidence_id))?;
    Ok(format!("{status:?}").to_uppercase())
}

#[tauri::command]
pub fn transcribe(
    state: State<'_, AppState>,
    evidence_id: String,
    model_path: String,
    use_gpu: bool,
) -> Result<TranscriptionOutcomeDto, CommandError> {
    let mut ws = state.workspace.lock()?;
    let cancel = cancel_token();
    let mut progress = |_fraction: Option<f32>| {};
    let outcome = ws.start_transcription(
        &EvidenceId::from_string(evidence_id),
        Path::new(&model_path),
        use_gpu,
        cancel,
        &mut progress,
    )?;
    Ok(TranscriptionOutcomeDto {
        transcript: TranscriptDto::from(&outcome.transcript),
        segments: outcome.segments.iter().map(SegmentDto::from).collect(),
        job: crate::dto::JobDto::from(&outcome.job),
        skipped_segments: outcome.skipped_segments,
    })
}

#[tauri::command]
pub fn transcript_for_evidence(
    state: State<'_, AppState>,
    evidence_id: String,
) -> Result<Option<TranscriptDto>, CommandError> {
    let ws = state.workspace.lock()?;
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
    let ws = state.workspace.lock()?;
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
    let mut ws = state.workspace.lock()?;
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
    let ws = state.workspace.lock()?;
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
