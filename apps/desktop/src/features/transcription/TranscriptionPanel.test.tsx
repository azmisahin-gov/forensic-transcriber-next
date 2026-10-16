import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { TranscriptionPanel } from "./TranscriptionPanel";
import { makeController } from "../../test/fakeController";

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

  it("disables start until a model path is provided", () => {
    const controller = makeController();
    render(<TranscriptionPanel controller={controller} selected={null} />);
    expect(
      screen.getByRole("button", { name: /Transkripsiyon başlat/i }),
    ).toBeDisabled();
  });
});
