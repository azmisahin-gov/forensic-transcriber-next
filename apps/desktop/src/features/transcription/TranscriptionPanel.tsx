import { useState } from "react";

import type { WorkspaceController } from "../../hooks/useWorkspace";
import type { EvidenceDto } from "../../types";

/**
 * Start, monitor and cancel a real transcription job. Engine and model are
 * detected or installed automatically; the user never types a path.
 */
export function TranscriptionPanel({
  controller,
  selected,
}: {
  controller: WorkspaceController;
  selected: EvidenceDto | null;
}) {
  const { busy, transcript, activeJobId, progress, startup } = controller;
  const [useGpu, setUseGpu] = useState(false);

  const engineReady = Boolean(startup?.engineAvailable);
  const modelReady = Boolean(startup?.modelAvailable);
  const ready = engineReady && modelReady;

  const statusText = ready
    ? `Motor hazır: whisper.cpp${startup?.engineVersion ? ` (${startup.engineVersion})` : ""} · Model: ${
        startup?.modelPath?.split(/[\\/]/).pop() ?? "hazır"
      }`
    : !engineReady
      ? "Motor kurulu değil. Yukarıdaki hazırlık bölümünden indirin."
      : "Model indirilmedi. Yukarıdaki hazırlık bölümünden indirin.";

  return (
    <section className="panel transcription-panel" aria-labelledby="transcription-heading">
      <h2 id="transcription-heading">Yazıya çevir</h2>

      <p className={ready ? "engine-status ok" : "engine-status warn"}>{statusText}</p>

      <div className="row">
        <label className="checkbox">
          <input
            type="checkbox"
            checked={useGpu}
            onChange={(e) => setUseGpu(e.target.checked)}
            disabled={busy}
          />
          GPU kullan (mevcutsa)
        </label>
        <button
          type="button"
          className="primary"
          disabled={busy || !selected || !ready}
          onClick={() =>
            selected && void controller.transcribe(selected.id, null, useGpu, "tr")
          }
        >
          Yazıya çevir
        </button>
        <button
          type="button"
          disabled={!activeJobId}
          onClick={() => void controller.cancelTranscription()}
        >
          İptal
        </button>
      </div>

      {!selected ? (
        <p className="muted">Yazıya çevirmek için önce bir kanıt seçin.</p>
      ) : null}

      {activeJobId ? (
        <div className="progress" role="progressbar" aria-label="Transkripsiyon ilerlemesi">
          <div
            className={progress === null ? "progress-bar indeterminate" : "progress-bar"}
            style={progress === null ? undefined : { width: `${Math.round(progress * 100)}%` }}
          />
          <span className="progress-label">
            {progress === null
              ? "İlerleme bilgisi bekleniyor…"
              : `%${Math.round(progress * 100)}`}
          </span>
        </div>
      ) : null}

      {transcript ? (
        <p className="engine-status ok">
          Transkript hazır · Motor: {transcript.engine ?? "—"} · Model:{" "}
          {transcript.modelId ?? "—"}
        </p>
      ) : null}
    </section>
  );
}
