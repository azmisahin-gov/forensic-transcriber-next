import { useEffect, useState } from "react";

import type { WorkspaceController } from "../../hooks/useWorkspace";
import { api } from "../../services/ipc";
import type { EngineProbeDto, EvidenceDto } from "../../types";

/** Transcribe the selected evidence and show engine availability honestly. */
export function TranscriptionPanel({
  controller,
  selected,
}: {
  controller: WorkspaceController;
  selected: EvidenceDto | null;
}) {
  const [modelPath, setModelPath] = useState("");
  const [useGpu, setUseGpu] = useState(false);
  const [probe, setProbe] = useState<EngineProbeDto | null>(null);
  const { busy, transcript, activeJobId, progress } = controller;

  useEffect(() => {
    api
      .engineProbe()
      .then(setProbe)
      .catch(() => setProbe({ available: false, version: null, note: "Sorgulanamadı" }));
  }, []);

  return (
    <section className="panel transcription-panel" aria-labelledby="transcription-heading">
      <h2 id="transcription-heading">Transkripsiyon</h2>

      {probe ? (
        <p className={probe.available ? "engine-status ok" : "engine-status warn"}>
          Motor: whisper.cpp —{" "}
          {probe.available
            ? `hazır${probe.version ? ` (${probe.version})` : ""}`
            : `kullanılamıyor${probe.note ? ` — ${probe.note}` : ""}`}
        </p>
      ) : (
        <p className="engine-status">Motor durumu kontrol ediliyor…</p>
      )}

      <div className="row">
        <label className="sr-only" htmlFor="model-path">
          Model dosyası yolu
        </label>
        <input
          id="model-path"
          type="text"
          placeholder="Model yolu (ör. C:\\modeller\\ggml-large-v3-turbo-q5_0.bin)"
          value={modelPath}
          onChange={(e) => setModelPath(e.target.value)}
          disabled={busy}
        />
      </div>

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
          disabled={busy || !selected || modelPath.trim().length === 0}
          onClick={() =>
            selected &&
            void controller.transcribe(selected.id, modelPath.trim(), useGpu)
          }
        >
          Transkripsiyon başlat
        </button>
        <button
          type="button"
          disabled={!activeJobId}
          onClick={() => void controller.cancelTranscription()}
        >
          İptal
        </button>
        <button
          type="button"
          disabled={busy || !selected}
          onClick={() => selected && void controller.verifyEvidence(selected.id)}
        >
          Bütünlüğü doğrula
        </button>
      </div>

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
          Motor: {transcript.engine ?? "—"} · Model: {transcript.modelId ?? "—"}
        </p>
      ) : null}
    </section>
  );
}
