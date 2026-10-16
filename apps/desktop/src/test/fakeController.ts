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
    notice: null,
    refreshCases: vi.fn().mockResolvedValue(undefined),
    createCase: vi.fn().mockResolvedValue(undefined),
    openCase: vi.fn().mockResolvedValue(undefined),
    importEvidence: vi.fn().mockResolvedValue(undefined),
    verifyEvidence: vi.fn().mockResolvedValue(undefined),
    transcribe: vi.fn().mockResolvedValue(undefined),
    cancelTranscription: vi.fn().mockResolvedValue(undefined),
    editSegment: vi.fn().mockResolvedValue(undefined),
    exportTranscript: vi.fn().mockResolvedValue(undefined),
    dismissNotice: vi.fn(),
  };
  return { ...base, ...overrides };
}
