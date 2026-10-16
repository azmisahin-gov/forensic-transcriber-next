import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StartupBanner } from "./StartupBanner";
import type { StartupStatusDto } from "../services/ipc";

describe("StartupBanner", () => {
  it("is hidden when startup is ok", () => {
    const ok: StartupStatusDto = {
      ok: true,
      error: null,
      dataDir: "C:/data",
      engineAvailable: false,
      engineVersion: null,
      engineNote: null,
    };
    const { container } = render(<StartupBanner startup={ok} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the error and data directory on failure", () => {
    const failed: StartupStatusDto = {
      ok: false,
      error: "disk error: access denied",
      dataDir: "C:/locked",
      engineAvailable: false,
      engineVersion: null,
      engineNote: null,
    };
    render(<StartupBanner startup={failed} />);
    expect(screen.getByRole("alert")).toHaveTextContent(/başlatılamadı/i);
    expect(screen.getByText(/access denied/)).toBeInTheDocument();
    expect(screen.getByText("C:/locked")).toBeInTheDocument();
  });
});
