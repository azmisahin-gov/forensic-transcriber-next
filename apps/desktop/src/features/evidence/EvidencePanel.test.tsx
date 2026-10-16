import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EvidencePanel } from "./EvidencePanel";
import { makeController } from "../../test/fakeController";
import type { CaseDto, EvidenceDto } from "../../types";

vi.mock("../../services/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../services/ipc")>();
  return {
    ...actual,
    pickAudioFile: vi.fn().mockResolvedValue("C:/secilen/ornek.wav"),
  };
});

import { pickAudioFile } from "../../services/ipc";

function caseDto(): CaseDto {
  return {
    id: "c1",
    title: "Vaka",
    notes: "",
    createdAt: "2026-10-16T09:00:00.000Z",
    updatedAt: "2026-10-16T09:30:00.000Z",
  };
}

function evidenceDto(): EvidenceDto {
  return {
    id: "e1",
    caseId: "c1",
    originalName: "ornek.wav",
    storedName: "ornek.wav",
    originalPath: "C:/secilen/ornek.wav",
    storedPath: "C:/data/cases/c1/evidence/original/ornek.wav",
    sizeBytes: 64078,
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

describe("EvidencePanel", () => {
  it("imports the file chosen from the native picker", async () => {
    const importEvidence = vi.fn().mockResolvedValue(undefined);
    const controller = makeController({ activeCase: caseDto(), importEvidence });
    render(
      <EvidencePanel controller={controller} selectedId={null} onSelect={vi.fn()} />,
    );

    await userEvent.click(screen.getByRole("button", { name: /Dosya seç ve içe aktar/i }));

    expect(pickAudioFile).toHaveBeenCalled();
    expect(importEvidence).toHaveBeenCalledWith("C:/secilen/ornek.wav");
  });

  it("disables the picker until a case is open", () => {
    const controller = makeController({ activeCase: null });
    render(
      <EvidencePanel controller={controller} selectedId={null} onSelect={vi.fn()} />,
    );
    expect(
      screen.getByRole("button", { name: /Dosya seç ve içe aktar/i }),
    ).toBeDisabled();
  });

  it("shows the selected evidence status", () => {
    const items = [evidenceDto()];
    const controller = makeController({ activeCase: caseDto(), evidence: items });
    render(
      <EvidencePanel controller={controller} selectedId="e1" onSelect={vi.fn()} />,
    );
    expect(screen.getByText(/Seçili kanıt/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Doğrulandı/).length).toBeGreaterThan(0);
    expect(screen.getByText(/16000 Hz/)).toBeInTheDocument();
  });
});
