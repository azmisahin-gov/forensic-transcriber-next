import { useState } from "react";

import type { WorkspaceController } from "../../hooks/useWorkspace";
import type { EvidenceDto } from "../../types";
import { formatBytes, integrityLabel } from "./format";

/** Evidence list, import-by-path and integrity verification. */
export function EvidencePanel({
  controller,
  selectedId,
  onSelect,
}: {
  controller: WorkspaceController;
  selectedId: string | null;
  onSelect: (evidence: EvidenceDto) => void;
}) {
  const [path, setPath] = useState("");
  const { evidence, activeCase, busy } = controller;

  function submit(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = path.trim();
    if (!trimmed) return;
    void controller.importEvidence(trimmed);
  }

  return (
    <section className="panel evidence-panel" aria-labelledby="evidence-heading">
      <h2 id="evidence-heading">Kanıt / Medya</h2>

      <form className="row" onSubmit={submit}>
        <label className="sr-only" htmlFor="evidence-path">
          Ses dosyası yolu
        </label>
        <input
          id="evidence-path"
          type="text"
          placeholder="Ses dosyasının tam yolu (ör. C:\\kayitlar\\ornek.wav)"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          disabled={busy || !activeCase}
        />
        <button type="submit" disabled={busy || !activeCase || path.trim().length === 0}>
          İçe aktar
        </button>
      </form>

      {!activeCase ? (
        <p className="empty">Kanıt eklemek için önce bir vaka açın.</p>
      ) : evidence.length === 0 ? (
        <p className="empty">Bu vakada henüz kanıt yok.</p>
      ) : (
        <ul className="evidence-list">
          {evidence.map((e) => (
            <li key={e.id}>
              <button
                type="button"
                className={e.id === selectedId ? "evidence-item active" : "evidence-item"}
                aria-current={e.id === selectedId}
                onClick={() => onSelect(e)}
              >
                <span className="evidence-name">{e.originalName}</span>
                <span className="evidence-meta">
                  {formatBytes(e.sizeBytes)}
                  {e.metadata.durationSeconds !== null
                    ? ` · ${e.metadata.durationSeconds.toFixed(1)} sn`
                    : ""}
                  {e.metadata.sampleRate !== null
                    ? ` · ${e.metadata.sampleRate} Hz`
                    : ""}
                  {e.metadata.channels !== null ? ` · ${e.metadata.channels} kanal` : ""}
                </span>
                <span
                  className={`badge integrity-${e.integrity.toLowerCase()}`}
                  title={`SHA-256: ${e.sourceSha256}`}
                >
                  {integrityLabel(e.integrity)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
