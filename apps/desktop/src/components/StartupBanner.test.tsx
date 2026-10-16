import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StartupBanner } from "./StartupBanner";
import type { StartupStatusDto } from "../services/ipc";

function status(overrides: Partial<StartupStatusDto>): StartupStatusDto {
  return {
    ok: true,
    error: null,
    dataDir: "C:/data",
    engineAvailable: false,
    engineVersion: null,
    engineNote: null,
    enginePath: null,
    engineVersionLabel: "v1.9.2",
    modelAvailable: false,
    modelPath: null,
    modelsDir: "C:/data/models",
    models: [],
    ...overrides,
  };
}

describe("StartupBanner", () => {
  it("is hidden when startup is ok", () => {
    const { container } = render(<StartupBanner startup={status({})} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the error and data directory on failure", () => {
    const failed = status({
      ok: false,
      error: "disk error: access denied",
      dataDir: "C:/locked",
    });
    render(<StartupBanner startup={failed} />);
    expect(screen.getByRole("alert")).toHaveTextContent(/başlatılamadı/i);
    expect(screen.getByText(/access denied/)).toBeInTheDocument();
    expect(screen.getByText("C:/locked")).toBeInTheDocument();
  });
});
