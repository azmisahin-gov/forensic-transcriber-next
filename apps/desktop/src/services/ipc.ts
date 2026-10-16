// The only bridge to the Rust core. Every call goes through `invoke`; there is
// no direct filesystem or SQLite access from the browser context.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  CaseDto,
  EngineProbeDto,
  EvidenceDto,
  ExportRecordDto,
  JobDto,
  SegmentDto,
  TranscriptDto,
  TranscriptionOutcomeDto,
} from "../types";

/** A normalised, safe error for the UI. */
export interface ApiError {
  code: string;
  message: string;
}

/** Type guard for the Rust error shape `{ code, message }`. */
function isApiError(value: unknown): value is ApiError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as Record<string, unknown>)["code"] === "string" &&
    typeof (value as Record<string, unknown>)["message"] === "string"
  );
}

function normaliseError(error: unknown): ApiError {
  if (isApiError(error)) {
    return { code: error.code, message: error.message };
  }
  if (typeof error === "string") {
    return { code: "ERROR", message: error };
  }
  return { code: "ERROR", message: "Beklenmeyen bir hata oluştu." };
}

async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normaliseError(error);
  }
}

export const api = {
  listCases: () => call<CaseDto[]>("list_cases"),
  createCase: (title: string) => call<CaseDto>("create_case", { title }),
  getCase: (caseId: string) => call<CaseDto>("get_case", { caseId }),

  listEvidence: (caseId: string) =>
    call<EvidenceDto[]>("list_evidence", { caseId }),
  importEvidence: (caseId: string, path: string) =>
    call<EvidenceDto>("import_evidence", { caseId, path }),
  verifyEvidence: (evidenceId: string) =>
    call<string>("verify_evidence", { evidenceId }),

  transcribe: (evidenceId: string, modelPath: string, useGpu: boolean) =>
    call<TranscriptionOutcomeDto>("transcribe", {
      evidenceId,
      modelPath,
      useGpu,
    }),
  cancelTranscription: (jobId: string) =>
    call<boolean>("cancel_transcription", { jobId }),
  jobStatus: (jobId: string) => call<JobDto | null>("job_status", { jobId }),
  transcriptForEvidence: (evidenceId: string) =>
    call<TranscriptDto | null>("transcript_for_evidence", { evidenceId }),
  listSegments: (transcriptId: string) =>
    call<SegmentDto[]>("list_segments", { transcriptId }),
  editSegment: (segmentId: string, text: string) =>
    call<SegmentDto>("edit_segment", { segmentId, text }),

  exportTranscript: (
    caseId: string,
    transcriptId: string,
    format: string,
  ) =>
    call<ExportRecordDto>("export_transcript", {
      caseId,
      transcriptId,
      format,
    }),

  engineProbe: () => call<EngineProbeDto>("engine_probe"),
};

/** Progress event payload emitted by the Rust core. */
export interface ProgressEvent {
  jobId: string;
  fraction: number | null;
}

/** Lifecycle event payload (`running` / `finished`). */
export interface StageEvent {
  jobId: string;
  stage: string;
}

/** Subscribe to transcription progress. Returns an unlisten function. */
export function onTranscriptionProgress(
  handler: (event: ProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ProgressEvent>("transcription:progress", (event) =>
    handler(event.payload),
  );
}

/** Subscribe to transcription stage changes. Returns an unlisten function. */
export function onTranscriptionStage(
  handler: (event: StageEvent) => void,
): Promise<UnlistenFn> {
  return listen<StageEvent>("transcription:stage", (event) =>
    handler(event.payload),
  );
}

/** Format a whole number of milliseconds as `mm:ss.mmm`. */
export function formatTime(ms: number): string {
  const clamped = Math.max(0, Math.floor(ms));
  const minutes = Math.floor(clamped / 60000);
  const seconds = Math.floor((clamped % 60000) / 1000);
  const millis = clamped % 1000;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}.${String(millis).padStart(3, "0")}`;
}
