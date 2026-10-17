import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

vi.mock("../../services/updater", () => ({
  currentVersion: vi.fn().mockResolvedValue("0.1.0"),
  checkForUpdate: vi.fn(),
  downloadAndInstall: vi.fn().mockResolvedValue(undefined),
  discardUpdate: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: vi.fn().mockResolvedValue(undefined),
}));

import { UpdatePanel } from "./UpdatePanel";
import { checkForUpdate, downloadAndInstall } from "../../services/updater";

const candidate = {
  update: {} as never,
  info: {
    currentVersion: "0.1.0",
    version: "0.2.0",
    notes: "- düzeltme",
    date: null,
  },
};

describe("UpdatePanel", () => {
  it("shows the current version and a check button", async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(null);
    render(<UpdatePanel startupDataDir="C:/data" />);
    expect(await screen.findByText("0.1.0")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Güncellemeyi denetle/i })).toBeInTheDocument();
  });

  it("offers install and postpone when an update is available", async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(candidate);
    render(<UpdatePanel startupDataDir={null} />);
    await userEvent.click(screen.getByRole("button", { name: /Güncellemeyi denetle/i }));
    const install = await screen.findByRole("button", { name: /indir ve kur/i });
    expect(install).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Ertele/i })).toBeInTheDocument();

    await userEvent.click(install);
    expect(downloadAndInstall).toHaveBeenCalled();
  });
});
