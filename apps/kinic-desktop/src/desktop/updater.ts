// Where: desktop updater integration.
// What: wraps Tauri updater and process plugins for manual update checks.
// Why: keep release transport logic out of Settings presentation code.

import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { DownloadEvent, Update } from "@tauri-apps/plugin-updater";

export type UpdateCheckResult =
  | { kind: "none"; message: string }
  | { kind: "available"; update: Update; version: string; currentVersion: string; notes: string | null };

export type UpdateProgress = {
  downloadedBytes: number;
  totalBytes: number | null;
};

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  const update = await check({ timeout: 30_000 });
  if (!update) {
    return { kind: "none", message: "No updates available." };
  }
  return {
    kind: "available",
    update,
    version: update.version,
    currentVersion: update.currentVersion,
    notes: update.body ?? null,
  };
}

export async function installUpdate(update: Update, onProgress: (progress: UpdateProgress) => void) {
  let downloadedBytes = 0;
  let totalBytes: number | null = null;
  await update.downloadAndInstall((event: DownloadEvent) => {
    switch (event.event) {
      case "Started":
        totalBytes = event.data.contentLength ?? null;
        onProgress({ downloadedBytes, totalBytes });
        break;
      case "Progress":
        downloadedBytes += event.data.chunkLength;
        onProgress({ downloadedBytes, totalBytes });
        break;
      case "Finished":
        onProgress({ downloadedBytes, totalBytes });
        break;
    }
  });
  await relaunch();
}
