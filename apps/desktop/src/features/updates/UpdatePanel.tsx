import { useCallback, useEffect, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";

import {
  checkForUpdate,
  currentVersion,
  discardUpdate,
  downloadAndInstall,
  type UpdateCandidate,
} from "../../services/updater";
import type { Update } from "@tauri-apps/plugin-updater";

type Phase = "idle" | "checking" | "available" | "downloading" | "error";

/**
 * About + update panel. It checks once shortly after launch (without forcing
 * anything on the user) and offers a manual check. The user must confirm the
 * download; there is no surprise restart, and the update can be postponed.
 */
export function UpdatePanel({ startupDataDir }: { startupDataDir: string | null }) {
  const [version, setVersion] = useState<string>("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [handle, setHandle] = useState<Update | null>(null);
  const [candidate, setCandidate] = useState<UpdateCandidate | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    currentVersion().then(setVersion).catch(() => undefined);
  }, []);

  const runCheck = useCallback(async (silent: boolean) => {
    if (!silent) setPhase("checking");
    setMessage(null);
    try {
      const found = await checkForUpdate();
      if (found) {
        setHandle(found.update);
        setCandidate(found.info);
        setPhase("available");
        setMessage(`Yeni sürüm mevcut: ${found.info.version}`);
      } else {
        setHandle(null);
        setCandidate(null);
        setPhase("idle");
        if (!silent) setMessage("Uygulama güncel.");
      }
    } catch (error) {
      // A missing endpoint or offline machine is not an error the user must act on.
      setPhase("idle");
      if (!silent) setMessage(`Güncelleme denetlenemedi: ${String(error)}`);
    }
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => void runCheck(true), 4000);
    return () => window.clearTimeout(timer);
  }, [runCheck]);

  const startDownload = useCallback(async () => {
    if (!handle) return;
    setPhase("downloading");
    setProgress(null);
    try {
      await downloadAndInstall(handle, setProgress);
      // On Windows the installer exits the app; other platforms need a relaunch.
      await relaunch();
    } catch (error) {
      setPhase("error");
      setMessage(`Güncelleme kurulamadı: ${String(error)}`);
    }
  }, [handle]);

  const postpone = useCallback(async () => {
    if (handle) await discardUpdate(handle);
    setHandle(null);
    setCandidate(null);
    setPhase("idle");
    setMessage("Güncelleme ertelendi. Uygulama çalışmaya devam ediyor.");
  }, [handle]);

  return (
    <section className="panel update-panel" aria-labelledby="update-heading">
      <h2 id="update-heading">Hakkında ve güncellemeler</h2>
      <p className="muted">
        Sürüm: <span className="mono">{version || "—"}</span>
        {startupDataDir ? (
          <>
            {" · Veri dizini: "}
            <span className="mono">{startupDataDir}</span>
          </>
        ) : null}
      </p>

      <div className="row">
        <button
          type="button"
          disabled={phase === "checking" || phase === "downloading"}
          onClick={() => void runCheck(false)}
        >
          {phase === "checking" ? "Denetleniyor…" : "Güncellemeyi denetle"}
        </button>

        {phase === "available" ? (
          <>
            <button type="button" className="primary" onClick={() => void startDownload()}>
              Sürüm {candidate?.version} indir ve kur
            </button>
            <button type="button" onClick={() => void postpone()}>
              Ertele
            </button>
          </>
        ) : null}
      </div>

      {phase === "downloading" ? (
        <div className="progress" role="progressbar" aria-label="Güncelleme indirme">
          <div
            className={progress === null ? "progress-bar indeterminate" : "progress-bar"}
            style={progress === null ? undefined : { width: `${Math.round(progress * 100)}%` }}
          />
          <span className="progress-label">
            {progress === null ? "İndiriliyor…" : `%${Math.round(progress * 100)}`}
          </span>
        </div>
      ) : null}

      {candidate?.notes ? (
        <details className="update-notes">
          <summary>Sürüm notları</summary>
          <pre>{candidate.notes}</pre>
        </details>
      ) : null}

      {message ? <p className="engine-status">{message}</p> : null}

      <p className="prep-note">
        Güncellemeler imzalıdır ve Tauri güncelleyicisi tarafından doğrulanır. Yeni
        bir sürüm, vaka verilerinizi ve indirdiğiniz modeli değiştirmez. Windows
        yükleyicisi kod imzalı değildir; SmartScreen uyarabilir.
      </p>
    </section>
  );
}
