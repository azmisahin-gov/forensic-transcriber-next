import type { WorkspaceController } from "../../hooks/useWorkspace";
import { formatBytes } from "../evidence/format";

/**
 * First-run preparation. It detects what is missing and installs it in place,
 * so a non-technical user never has to find a binary, name a model file or set
 * an environment variable. Downloads verify a pinned SHA-256 before use.
 */
export function PreparationPanel({ controller }: { controller: WorkspaceController }) {
  const { startup, runtimeBusy, runtimeProgress } = controller;

  if (!startup || !startup.ok) {
    return null;
  }
  if (startup.engineAvailable && startup.modelAvailable) {
    return null;
  }

  const pct =
    runtimeProgress === null ? null : Math.round(runtimeProgress * 100);

  return (
    <section className="panel preparation" aria-labelledby="prep-heading">
      <h2 id="prep-heading">İlk kurulum: transkripsiyon hazırlığı</h2>
      <p>
        Vaka oluşturma, ses içe aktarma ve <strong>SHA-256 bütünlük doğrulama</strong>{" "}
        motor olmadan da çalışır. Otomatik Türkçe transkripsiyon için aşağıdaki iki
        bileşen gerekir. İnternet yalnızca bu indirmeler için kullanılır; ses
        kayıtlarınız hiçbir zaman uzak bir servise gönderilmez.
      </p>

      <ol className="prep-steps">
        <li className={startup.engineAvailable ? "done" : ""}>
          <strong>whisper.cpp motoru</strong>{" "}
          {startup.engineAvailable ? (
            <span className="ok-text">kurulu</span>
          ) : (
            <button
              type="button"
              className="primary"
              disabled={runtimeBusy}
              onClick={() => void controller.prepareEngine()}
            >
              Motoru indir ({startup.engineVersionLabel})
            </button>
          )}
        </li>
        <li className={startup.modelAvailable ? "done" : ""}>
          <strong>Konuşma modeli</strong>{" "}
          {startup.modelAvailable ? (
            <span className="ok-text">indirildi</span>
          ) : (
            <span className="model-options">
              {startup.models.map((model) => (
                <button
                  key={model.id}
                  type="button"
                  className="primary"
                  disabled={runtimeBusy || model.installed}
                  onClick={() => void controller.downloadModel(model.id)}
                >
                  {model.installed
                    ? `${model.label} — indirildi`
                    : `${model.label} (${formatBytes(model.sizeBytes)}) indir`}
                </button>
              ))}
            </span>
          )}
        </li>
      </ol>

      {runtimeBusy ? (
        <div className="row">
          <div className="progress" role="progressbar" aria-label="İndirme ilerlemesi">
            <div
              className={pct === null ? "progress-bar indeterminate" : "progress-bar"}
              style={pct === null ? undefined : { width: `${pct}%` }}
            />
            <span className="progress-label">
              {pct === null ? "İndiriliyor…" : `%${pct}`}
            </span>
          </div>
          <button type="button" onClick={() => void controller.cancelPreparation()}>
            İptal
          </button>
        </div>
      ) : null}

      <p className="prep-note">
        Model dosyası: <span className="mono">{startup.modelsDir}</span>. İndirilen
        model bir kez doğrulandıktan sonra çevrimdışı çalışır.
      </p>
      {startup.engineNote ? (
        <p className="muted mono">Tanılama: {startup.engineNote}</p>
      ) : null}
    </section>
  );
}
