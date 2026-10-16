import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ErrorBoundary } from "./ErrorBoundary";

function Bomb(): JSX.Element {
  throw new Error("patlama testi");
}

describe("ErrorBoundary", () => {
  it("renders children when there is no error", () => {
    render(
      <ErrorBoundary>
        <span>sağlam içerik</span>
      </ErrorBoundary>,
    );
    expect(screen.getByText("sağlam içerik")).toBeInTheDocument();
  });

  it("renders a readable Turkish fallback on error", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(/beklenmeyen bir hata/i);
    expect(screen.getByText("patlama testi")).toBeInTheDocument();
    spy.mockRestore();
  });
});
