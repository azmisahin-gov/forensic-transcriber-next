// Data-transfer types mirrored from the Rust Tauri command surface.
// Times arrive as ISO-8601 strings; millisecond values are kept for alignment.

export interface CaseDto {
  id: string;
  title: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface MediaMetadataDto {
  format: string | null;
  codec: string | null;
  durationSeconds: number | null;
  sampleRate: number | null;
  channels: number | null;
  bitDepth: number | null;
  audioStreamCount: number | null;
}

export type IntegrityStatus =
  | "VERIFIED"
  | "MISMATCH"
  | "MISSING"
  | "UNCHECKED";

export interface EvidenceDto {
  id: string;
  caseId: string;
  originalName: string;
  storedName: string;
  originalPath: string;
  storedPath: string;
  sizeBytes: number;
  metadata: MediaMetadataDto;
  sourceSha256: string;
  storedSha256: string;
  integrity: IntegrityStatus;
  importedAt: string;
}

export type SegmentStatus = "AUTOMATIC" | "REVIEWED" | "EDITED" | "VERIFIED";

export interface SegmentDto {
  id: string;
  transcriptId: string;
  ordinal: number;
  startMs: number;
  endMs: number;
  speaker: string;
  text: string;
  originalText: string;
  status: SegmentStatus;
  confidence: number | null;
  flags: string[];
  updatedAt: string;
}

export interface TranscriptDto {
  id: string;
  evidenceId: string;
  language: string;
  engine: string | null;
  modelId: string | null;
  createdAt: string;
  updatedAt: string;
}

export type JobStatus =
  | "QUEUED"
  | "RUNNING"
  | "SUCCEEDED"
  | "FAILED"
  | "CANCELLED";

export interface JobDto {
  id: string;
  evidenceId: string;
  transcriptId: string | null;
  status: JobStatus;
  errorCode: string | null;
  errorMessage: string | null;
  startedAt: string;
  finishedAt: string | null;
}

export interface TranscriptionOutcomeDto {
  transcript: TranscriptDto;
  segments: SegmentDto[];
  job: JobDto;
  skippedSegments: number;
}

export interface ExportRecordDto {
  id: string;
  format: string;
  path: string;
  sha256: string | null;
  schemaVersion: string;
  createdAt: string;
}

export interface EngineProbeDto {
  available: boolean;
  version: string | null;
  note: string | null;
}
