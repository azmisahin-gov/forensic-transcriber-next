import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { PreparationPanel } from "./PreparationPanel";
import { makeController } from "../../test/fakeController";
import type { StartupStatusDto } from "../../services/ipc";

function status(engineAvailable: boolean, modelAvailable: boolean): StartupStatusDto {
  return {
    ok: true,
    error: null,
    dataDir: "C:/data",
    engineAvailable,
    engineVersion: engineAvailable ? "1.9.2" : null,
    engineNote: engineAvailable ? null : "whisper-cli bulunamadı",
    enginePath: engineAvailable ? "C:/data/runtime/whisper/Release/whisper-cli.exe" : null,
    engineVersionLabel: "v1.9.2",
    modelAvailable,
    modelPath: modelAvailable ? "C:/data/models/ggml-base.bin" : null,
    modelsDir: "C:/data/models",
    models: [
      { id: "base", label: "Base", sizeBytes: 147951465, installed: modelAvailable },
    ],
  };
}

describe("PreparationPanel", () => {
  it("is hidden when everything is ready", () => {
    const controller = makeController({ startup: status(true, true) });
    const { container } = render(<PreparationPanel controller={controller} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("offers the engine download when the engine is missing", async () => {
    const prepareEngine = vi.fn().mockResolvedValue(undefined);
    const controller = makeController({
      startup: status(false, false),
      prepareEngine,
    });
    render(<PreparationPanel controller={controller} />);

    expect(screen.getByText(/İlk kurulum/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Motoru indir/i }));
    expect(prepareEngine).toHaveBeenCalledTimes(1);
  });

  it("offers a model download when only the model is missing", async () => {
    const downloadModel = vi.fn().mockResolvedValue(undefined);
    const controller = makeController({
      startup: status(true, false),
      downloadModel,
    });
    render(<PreparationPanel controller={controller} />);

    const button = screen.getByRole("button", { name: /indir$/i });
    await userEvent.click(button);
    expect(downloadModel).toHaveBeenCalledWith("base");
  });

  it("renders nothing when startup status is unknown", () => {
    const { container } = render(<PreparationPanel controller={makeController()} />);
    expect(container).toBeEmptyDOMElement();
  });
});
