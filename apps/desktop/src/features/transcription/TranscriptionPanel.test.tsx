import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { TranscriptionPanel } from "./TranscriptionPanel";
import { makeController } from "../../test/fakeController";
import type { StartupStatusDto } from "../../services/ipc";
import type { EvidenceDto } from "../../types";

const readyStartup: StartupStatusDto = {
  ok: true,
  error: null,
  dataDir: "C:/data",
  engineAvailable: true,
  engineVersion: "1.9.2",
  engineNote: null,
  enginePath: "C:/data/runtime/whisper/Release/whisper-cli.exe",
  engineVersionLabel: "v1.9.2",
  modelAvailable: true,
  modelPath: "C:/data/models/ggml-base.bin",
  modelsDir: "C:/data/models",
  models: [],
};

function evidenceDto(): EvidenceDto {
  return {
    id: "e1",
    caseId: "c1",
    originalName: "ornek.wav",
    storedName: "ornek.wav",
    originalPath: "C:/secilen/ornek.wav",
    storedPath: "C:/data/ornek.wav",
    sizeBytes: 1000,
    metadata: {
      format: "wav",
      codec: "pcm_s16le",
      durationSeconds: 2,
      sampleRate: 16000,
      channels: 1,
      bitDepth: 16,
      audioStreamCount: 1,
    },
    sourceSha256: "a".repeat(64),
    storedSha256: "a".repeat(64),
    integrity: "VERIFIED",
    importedAt: "2026-10-16T09:00:00.000Z",
  };
}

describe("TranscriptionPanel", () => {
  it("disables cancel when no job is running", () => {
    render(<TranscriptionPanel controller={makeController()} selected={null} />);
    expect(screen.getByRole("button", { name: "İptal" })).toBeDisabled();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("shows progress and enables cancel while a job runs", async () => {
    const cancelTranscription = vi.fn().mockResolvedValue(undefined);
    const controller = makeController({
      activeJobId: "job-1",
      progress: 0.5,
      busy: true,
      cancelTranscription,
    });
    render(<TranscriptionPanel controller={controller} selected={null} />);

    expect(screen.getByRole("progressbar")).toBeInTheDocument();
    expect(screen.getByText("%50")).toBeInTheDocument();

    const cancel = screen.getByRole("button", { name: "İptal" });
    expect(cancel).toBeEnabled();
    await userEvent.click(cancel);
    expect(cancelTranscription).toHaveBeenCalledTimes(1);
  });

  it("disables transcription until an evidence is selected", () => {
    const controller = makeController({ startup: readyStartup });
    render(<TranscriptionPanel controller={controller} selected={null} />);
    expect(screen.getByRole("button", { name: /Yazıya çevir/i })).toBeDisabled();
  });

  it("enables transcription when the runtime is ready and evidence is selected", () => {
    const controller = makeController({ startup: readyStartup });
    render(<TranscriptionPanel controller={controller} selected={evidenceDto()} />);
    expect(screen.getByRole("button", { name: /Yazıya çevir/i })).toBeEnabled();
  });
});
