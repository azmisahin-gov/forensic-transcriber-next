// The only bridge to the Rust core. Every call goes through `invoke`; there is
// no direct filesystem or SQLite access from the browser context.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

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

  transcribe: (
    evidenceId: string,
    modelPath: string | null,
    useGpu: boolean,
    language?: string,
  ) =>
    call<TranscriptionOutcomeDto>("transcribe", {
      evidenceId,
      modelPath,
      useGpu,
      language,
    }),
  prepareEngine: () => call<StartupStatusDto>("prepare_engine"),
  downloadModel: (modelId: string) =>
    call<StartupStatusDto>("download_model", { modelId }),
  cancelPreparation: () => call<boolean>("cancel_preparation"),
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
  startupStatus: () => call<StartupStatusDto>("startup_status"),
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

/** Startup diagnostics reported by the Rust core. */
export interface ModelOption {
  id: string;
  label: string;
  sizeBytes: number;
  installed: boolean;
}

export interface StartupStatusDto {
  ok: boolean;
  error: string | null;
  dataDir: string;
  engineAvailable: boolean;
  engineVersion: string | null;
  engineNote: string | null;
  enginePath: string | null;
  engineVersionLabel: string;
  modelAvailable: boolean;
  modelPath: string | null;
  modelsDir: string;
  models: ModelOption[];
}

/** Runtime download progress payload. */
export interface RuntimeProgressEvent {
  stage: string;
  downloaded: number;
  total: number;
}

/** Extensions offered by the native file picker. */
const AUDIO_EXTENSIONS = [
  "wav",
  "mp3",
  "m4a",
  "m4b",
  "flac",
  "ogg",
  "oga",
  "opus",
  "aac",
  "wma",
  "aiff",
  "aif",
  "amr",
  "mp4",
  "mov",
  "mkv",
  "webm",
];

/**
 * Open the native file picker for an audio/video file. Returns the selected
 * absolute path, or null when the user cancels.
 */
export async function pickAudioFile(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Ses dosyası seç",
    filters: [{ name: "Ses / Video", extensions: AUDIO_EXTENSIONS }],
  });
  return typeof selected === "string" ? selected : null;
}

/**
 * Build a URL for the asset protocol that streams a managed evidence copy. Only
 * the app data directory is exposed, and the path always comes from the backend.
 */
export function mediaSrc(storedPath: string): string {
  return convertFileSrc(storedPath);
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

/** Subscribe to engine/model download progress. Returns an unlisten function. */
export function onRuntimeProgress(
  handler: (event: RuntimeProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<RuntimeProgressEvent>("runtime:progress", (event) =>
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
