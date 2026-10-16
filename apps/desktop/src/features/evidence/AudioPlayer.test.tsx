import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AudioPlayer } from "./AudioPlayer";

describe("AudioPlayer", () => {
  it("asks for a selection when no evidence is chosen", () => {
    render(<AudioPlayer evidence={null} />);
    expect(screen.getByText(/Oynatmak için bir kanıt seçin/i)).toBeInTheDocument();
  });
});
