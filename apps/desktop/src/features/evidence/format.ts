import type { IntegrityStatus } from "../../types";

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(1)} ${units[unitIndex] ?? "B"}`;
}

export function integrityLabel(status: IntegrityStatus): string {
  switch (status) {
    case "VERIFIED":
      return "Doğrulandı";
    case "MISMATCH":
      return "Uyuşmazlık";
    case "MISSING":
      return "Eksik";
    default:
      return "Denetlenmedi";
  }
}
