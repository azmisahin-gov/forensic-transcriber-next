import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PreparationPanel } from "./PreparationPanel";
import type { StartupStatusDto } from "../../services/ipc";

function startup(engineAvailable: boolean): StartupStatusDto {
  return {
    ok: true,
    error: null,
    dataDir: "C:/data",
    engineAvailable,
    engineVersion: engineAvailable ? "1.9.4" : null,
    engineNote: engineAvailable ? null : "whisper-cli not configured or not found",
  };
}

describe("PreparationPanel", () => {
  it("is hidden when an engine is available", () => {
    const { container } = render(<PreparationPanel startup={startup(true)} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows setup guidance when no engine is available", () => {
    render(<PreparationPanel startup={startup(false)} />);
    expect(screen.getByText(/Transkripsiyon motoru kurulu değil/i)).toBeInTheDocument();
    expect(screen.getByText(/FTN_ENGINE_PATH/)).toBeInTheDocument();
    expect(screen.getByText(/bütünlük doğrulama/i)).toBeInTheDocument();
    expect(screen.getByText(/doğrulanmamıştır/i)).toBeInTheDocument();
  });

  it("renders nothing when startup is unknown", () => {
    const { container } = render(<PreparationPanel startup={null} />);
    expect(container).toBeEmptyDOMElement();
  });
});
