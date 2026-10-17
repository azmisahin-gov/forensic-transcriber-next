// Updater bridge. The update check, download and install run in Rust via the
// official Tauri updater plugin; this module only shapes the data for the UI.
import { getVersion } from "@tauri-apps/api/app";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

/** What the UI needs to describe an available update. */
export interface UpdateCandidate {
  currentVersion: string;
  version: string;
  notes: string | null;
  date: string | null;
}

/** The running application version. */
export async function currentVersion(): Promise<string> {
  return getVersion();
}

/**
 * Check the configured endpoint for an update. Returns the live `Update` handle
 * alongside display data, or null when the app is up to date.
 */
export async function checkForUpdate(): Promise<{
  update: Update;
  info: UpdateCandidate;
} | null> {
  const update = await check();
  if (!update) {
    return null;
  }
  return {
    update,
    info: {
      currentVersion: update.currentVersion,
      version: update.version,
      notes: update.body ?? null,
      date: update.date ?? null,
    },
  };
}

/**
 * Download and install an update, reporting progress as a fraction in [0, 1]
 * (or null when the total size is unknown). On Windows the installer is
 * launched and the app exits; the installer restarts it.
 */
export async function downloadAndInstall(
  update: Update,
  onProgress: (fraction: number | null) => void,
): Promise<void> {
  let total = 0;
  let done = 0;
  await update.downloadAndInstall((event: DownloadEvent) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? 0;
      onProgress(total > 0 ? 0 : null);
    } else if (event.event === "Progress") {
      done += event.data.chunkLength;
      onProgress(total > 0 ? Math.min(1, done / total) : null);
    } else {
      onProgress(1);
    }
  });
}

/** Release the update handle without installing. */
export async function discardUpdate(update: Update): Promise<void> {
  try {
    await update.close();
  } catch {
    // Releasing an already-consumed handle is not an error for the user.
  }
}
