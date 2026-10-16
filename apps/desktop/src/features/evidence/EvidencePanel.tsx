import { useState } from "react";

import type { WorkspaceController } from "../../hooks/useWorkspace";
import { pickAudioFile } from "../../services/ipc";
import type { EvidenceDto } from "../../types";
import { formatBytes, integrityLabel } from "./format";

/** Evidence list, native file import and integrity verification. */
export function EvidencePanel({
  controller,
  selectedId,
  onSelect,
}: {
  controller: WorkspaceController;
  selectedId: string | null;
  onSelect: (evidence: EvidenceDto) => void;
}) {
  const [advanced, setAdvanced] = useState(false);
  const [path, setPath] = useState("");
  const { evidence, activeCase, busy } = controller;

  async function pickAndImport() {
    const selected = await pickAudioFile();
    if (selected) {
      await controller.importEvidence(selected);
    }
  }

  function submitPath(event: React.FormEvent) {
    event.preventDefault();
    const trimmed = path.trim();
    if (!trimmed) return;
    void controller.importEvidence(trimmed).then(() => setPath(""));
  }

  const selected = evidence.find((e) => e.id === selectedId) ?? null;

  return (
    <section className="panel evidence-panel" aria-labelledby="evidence-heading">
      <div className="panel-head">
        <h2 id="evidence-heading">Kanıt / Medya</h2>
        <button
          type="button"
          className="ghost"
          onClick={() => setAdvanced((v) => !v)}
        >
          {advanced ? "Yol girişini gizle" : "Gelişmiş: yol gir"}
        </button>
      </div>

      <div className="row">
        <button
          type="button"
          className="primary"
          onClick={() => void pickAndImport()}
          disabled={busy || !activeCase}
        >
          Dosya seç ve içe aktar
        </button>
        {!activeCase ? (
          <span className="muted">Önce bir vaka açın.</span>
        ) : null}
      </div>

      {advanced ? (
        <form className="row" onSubmit={submitPath}>
          <label className="sr-only" htmlFor="evidence-path">
            Ses dosyası yolu
          </label>
          <input
            id="evidence-path"
            type="text"
            placeholder="Ses dosyasının tam yolu"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            disabled={busy}
          />
          <button type="submit" disabled={busy || path.trim().length === 0}>
            İçe aktar
          </button>
        </form>
      ) : null}

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

      {selected ? <SelectedEvidence evidence={selected} controller={controller} /> : null}
    </section>
  );
}

function SelectedEvidence({
  evidence,
  controller,
}: {
  evidence: EvidenceDto;
  controller: WorkspaceController;
}) {
  return (
    <div className="evidence-detail" aria-label="Seçili kanıt ayrıntısı">
      <h3>Seçili kanıt</h3>
      <dl className="detail-grid">
        <dt>Dosya</dt>
        <dd>{evidence.originalName}</dd>
        <dt>Yönetilen kopya</dt>
        <dd className="mono">{evidence.storedName}</dd>
        <dt>Biçim / codec</dt>
        <dd>
          {evidence.metadata.format ?? "bilinmiyor"}
          {evidence.metadata.codec ? ` / ${evidence.metadata.codec}` : ""}
        </dd>
        <dt>Süre / kanal</dt>
        <dd>
          {evidence.metadata.durationSeconds !== null
            ? `${evidence.metadata.durationSeconds.toFixed(2)} sn`
            : "bilinmiyor"}
          {evidence.metadata.channels !== null ? ` · ${evidence.metadata.channels} kanal` : ""}
        </dd>
        <dt>Bütünlük</dt>
        <dd>
          <span className={`badge integrity-${evidence.integrity.toLowerCase()}`}>
            {integrityLabel(evidence.integrity)}
          </span>
        </dd>
        <dt>SHA-256</dt>
        <dd className="mono hash">{evidence.sourceSha256}</dd>
      </dl>
      <button
        type="button"
        onClick={() => void controller.verifyEvidence(evidence.id)}
      >
        Bütünlüğü yeniden doğrula
      </button>
    </div>
  );
}
