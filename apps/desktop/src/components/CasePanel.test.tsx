import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { CasePanel } from "./CasePanel";
import { makeController } from "../test/fakeController";
import type { CaseDto } from "../types";

function caseDto(id: string, title: string): CaseDto {
  return {
    id,
    title,
    notes: "",
    createdAt: "2026-10-16T09:00:00.000Z",
    updatedAt: "2026-10-16T09:30:00.000Z",
  };
}

describe("CasePanel", () => {
  it("shows an empty state when there are no cases", () => {
    render(<CasePanel controller={makeController()} />);
    expect(screen.getByText(/henüz vaka yok/i)).toBeInTheDocument();
  });

  it("creates a case from the form and clears the input", async () => {
    const createCase = vi.fn().mockResolvedValue(undefined);
    const controller = makeController({ createCase });
    render(<CasePanel controller={controller} />);

    const input = screen.getByLabelText(/yeni vaka başlığı/i);
    await userEvent.type(input, "Deneme Vaka");
    await userEvent.click(screen.getByRole("button", { name: /oluştur/i }));

    expect(createCase).toHaveBeenCalledWith("Deneme Vaka");
  });

  it("lists cases and marks the active one", () => {
    const cases = [caseDto("c1", "Birinci"), caseDto("c2", "İkinci")];
    const controller = makeController({ cases, activeCase: cases[1] ?? null });
    render(<CasePanel controller={controller} />);

    const active = screen.getByRole("button", { name: /İkinci/ });
    expect(active).toHaveAttribute("aria-current", "true");
  });

  it("opens a case when clicked", async () => {
    const openCase = vi.fn().mockResolvedValue(undefined);
    const cases = [caseDto("c1", "Birinci")];
    render(<CasePanel controller={makeController({ cases, openCase })} />);

    await userEvent.click(screen.getByRole("button", { name: /birinci/i }));
    expect(openCase).toHaveBeenCalledWith("c1");
  });
});
