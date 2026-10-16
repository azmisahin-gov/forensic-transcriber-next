import { vi } from "vitest";

import type { WorkspaceController } from "../hooks/useWorkspace";

/** Build a fully-stubbed controller; override only what a test needs. */
export function makeController(
  overrides: Partial<WorkspaceController> = {},
): WorkspaceController {
  const base: WorkspaceController = {
    cases: [],
    activeCase: null,
    evidence: [],
    transcript: null,
    segments: [],
    exports: [],
    busy: false,
    activeJobId: null,
    progress: null,
    startup: null,
    runtimeBusy: false,
    runtimeProgress: null,
    notice: null,
    refreshCases: vi.fn().mockResolvedValue(undefined),
    createCase: vi.fn().mockResolvedValue(undefined),
    openCase: vi.fn().mockResolvedValue(undefined),
    loadEvidenceTranscript: vi.fn().mockResolvedValue(undefined),
    importEvidence: vi.fn().mockResolvedValue(undefined),
    verifyEvidence: vi.fn().mockResolvedValue(undefined),
    transcribe: vi.fn().mockResolvedValue(undefined),
    cancelTranscription: vi.fn().mockResolvedValue(undefined),
    prepareEngine: vi.fn().mockResolvedValue(undefined),
    downloadModel: vi.fn().mockResolvedValue(undefined),
    cancelPreparation: vi.fn().mockResolvedValue(undefined),
    refreshStatus: vi.fn().mockResolvedValue(undefined),
    editSegment: vi.fn().mockResolvedValue(undefined),
    exportTranscript: vi.fn().mockResolvedValue(undefined),
    dismissNotice: vi.fn(),
  };
  return { ...base, ...overrides };
}
