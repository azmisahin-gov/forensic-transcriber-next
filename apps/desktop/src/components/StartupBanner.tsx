import type { StartupStatusDto } from "../services/ipc";

/**
 * Prominent banner shown when the workspace could not be opened at startup, so
 * the failure is visible instead of the app silently doing nothing.
 */
export function StartupBanner({ startup }: { startup: StartupStatusDto | null }) {
  if (!startup || startup.ok) {
    return null;
  }
  return (
    <div className="startup-banner" role="alert">
      <strong>Çalışma alanı başlatılamadı.</strong>
      <span>{startup.error ?? "Bilinmeyen hata."}</span>
      <span className="muted">
        Veri dizini: <span className="mono">{startup.dataDir}</span>
      </span>
    </div>
  );
}
