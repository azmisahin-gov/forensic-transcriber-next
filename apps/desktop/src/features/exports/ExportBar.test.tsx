import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ExportBar } from "./ExportBar";
import { makeController } from "../../test/fakeController";
import type { TranscriptDto } from "../../types";

function transcript(): TranscriptDto {
  return {
    id: "t1",
    evidenceId: "e1",
    language: "tr",
    engine: "whisper.cpp",
    modelId: "large-v3-turbo",
    createdAt: "2026-10-16T09:00:00.000Z",
    updatedAt: "2026-10-16T09:30:00.000Z",
  };
}

describe("ExportBar", () => {
  it("disables export buttons when there is no transcript", () => {
    render(<ExportBar controller={makeController()} />);
    for (const label of ["TXT", "SRT", "JSON", "HTML"]) {
      expect(
        screen.getByRole("button", { name: new RegExp(`${label} olarak`) }),
      ).toBeDisabled();
    }
  });

  it("exports the requested format when a transcript exists", async () => {
    const exportTranscript = vi.fn().mockResolvedValue(undefined);
    const controller = makeController({ transcript: transcript(), exportTranscript });
    render(<ExportBar controller={controller} />);

    await userEvent.click(screen.getByRole("button", { name: /SRT olarak/i }));
    expect(exportTranscript).toHaveBeenCalledWith("SRT");
  });
});
