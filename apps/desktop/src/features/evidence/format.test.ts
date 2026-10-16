import { describe, expect, it } from "vitest";

import { formatBytes, integrityLabel } from "./format";

describe("evidence formatting", () => {
  it("formats byte sizes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("labels integrity states in Turkish", () => {
    expect(integrityLabel("VERIFIED")).toBe("Doğrulandı");
    expect(integrityLabel("MISMATCH")).toBe("Uyuşmazlık");
    expect(integrityLabel("MISSING")).toBe("Eksik");
    expect(integrityLabel("UNCHECKED")).toBe("Denetlenmedi");
  });
});
