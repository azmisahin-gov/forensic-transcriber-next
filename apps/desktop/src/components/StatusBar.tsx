import type { Notice } from "../hooks/useWorkspace";

/** Bottom status strip: notices and busy state. */
export function StatusBar({
  notice,
  busy,
  onDismiss,
}: {
  notice: Notice | null;
  busy: boolean;
  onDismiss: () => void;
}) {
  return (
    <footer className="status-bar" role="status" aria-live="polite">
      {busy ? <span className="spinner" aria-hidden="true" /> : null}
      {busy ? <span>İşlem sürüyor…</span> : null}
      {notice ? (
        <span className={`notice notice-${notice.kind}`}>
          {notice.message}
          <button
            type="button"
            className="notice-dismiss"
            aria-label="Bildirimi kapat"
            onClick={onDismiss}
          >
            ×
          </button>
        </span>
      ) : null}
      <span className="status-hint">
        Otomatik çıktı uzman kanaati değildir; inceleme gerektirir.
      </span>
    </footer>
  );
}
