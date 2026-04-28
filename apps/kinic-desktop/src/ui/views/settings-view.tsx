// Where: Settings workflow.
// What: displays session and shared TUI preferences with default memory mutation.
// Why: Desktop should share local preferences with CLI/TUI instead of localStorage.

import { useRef, useState } from "react";
import type { Update } from "@tauri-apps/plugin-updater";
import { Button, Card, FieldLabel, Input, Metric, Badge } from "../primitives";
import { shortPrincipal, valueOrUnavailable } from "@/desktop/format";
import { checkForUpdate, installUpdate, type UpdateProgress } from "@/desktop/updater";
import { useDesktopStore } from "@/store/useDesktopStore";

export function SettingsView() {
  const session = useDesktopStore((state) => state.session);
  const preferences = useDesktopStore((state) => state.preferences);
  const memories = useDesktopStore((state) => state.memories);
  const loading = useDesktopStore((state) => state.loading);
  const updateDefaultMemory = useDesktopStore((state) => state.updateDefaultMemory);
  const pendingUpdate = useRef<Update | null>(null);
  const [updateMessage, setUpdateMessage] = useState("Not checked.");
  const [updateNotes, setUpdateNotes] = useState<string | null>(null);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [installingUpdate, setInstallingUpdate] = useState(false);

  async function handleCheckForUpdate() {
    setCheckingUpdate(true);
    setUpdateNotes(null);
    setUpdateProgress(null);
    pendingUpdate.current = null;
    try {
      const result = await checkForUpdate();
      if (result.kind === "none") {
        setUpdateMessage(result.message);
        return;
      }
      pendingUpdate.current = result.update;
      setUpdateMessage(`Update available: ${result.currentVersion} -> ${result.version}`);
      setUpdateNotes(result.notes);
    } catch (error) {
      setUpdateMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setCheckingUpdate(false);
    }
  }

  async function handleInstallUpdate() {
    const update = pendingUpdate.current;
    if (!update) {
      setUpdateMessage("Check for an update first.");
      return;
    }
    setInstallingUpdate(true);
    try {
      await installUpdate(update, setUpdateProgress);
      setUpdateMessage("Update installed. Relaunching...");
    } catch (error) {
      setUpdateMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setInstallingUpdate(false);
    }
  }

  return (
    <div className="grid max-w-6xl gap-5 xl:grid-cols-2">
      <Card className="p-5">
        <h2 className="text-xl font-semibold">Session</h2>
        <div className="mt-4 grid gap-3 md:grid-cols-2">
          <Metric label="Identity" value={valueOrUnavailable(session?.identity)} />
          <Metric label="Network" value={valueOrUnavailable(session?.network)} />
          <Metric label="Auth Mode" value={valueOrUnavailable(session?.auth_mode)} />
          <Metric label="Principal" value={shortPrincipal(session?.principal_id)} />
          <Metric label="Embedding" value={valueOrUnavailable(session?.embedding_api_endpoint)} />
          <Metric label="Balance" value={valueOrUnavailable(session?.balance_kinic)} />
        </div>
      </Card>

      <Card className="p-5">
        <h2 className="text-xl font-semibold">Shared Preferences</h2>
        <div className="mt-4 grid gap-5">
          <div className="grid gap-2">
            <FieldLabel htmlFor="default-memory">Default Memory</FieldLabel>
            <Input
              id="default-memory"
              value={preferences?.default_memory_id ?? ""}
              readOnly
              placeholder="Not set"
            />
          </div>
          <div className="grid gap-2">
            <FieldLabel>Set Default From Loaded Memories</FieldLabel>
            <div className="flex flex-wrap gap-2">
              {memories.map((memory) => {
                const memoryId = memory.searchable_memory_id ?? memory.id;
                return (
                  <Button
                    key={memory.id}
                    variant="secondary"
                    size="sm"
                    disabled={loading}
                    onClick={() => void updateDefaultMemory(memoryId)}
                  >
                    {memory.name}
                  </Button>
                );
              })}
              <Button variant="outline" size="sm" disabled={loading} onClick={() => void updateDefaultMemory(null)}>
                Clear
              </Button>
            </div>
          </div>
          <PreferenceBadges label="Saved Tags" values={preferences?.saved_tags ?? []} empty="No saved tags" />
          <PreferenceBadges label="Manual Memories" values={preferences?.manual_memory_ids ?? []} empty="No manual memories" />
          <div className="grid gap-3 md:grid-cols-3">
            <Metric label="Chat Result Limit" value={preferences?.chat_overall_top_k ?? "Unavailable"} />
            <Metric label="Per Memory Limit" value={preferences?.chat_per_memory_cap ?? "Unavailable"} />
            <Metric label="Chat Diversity" value={preferences?.chat_mmr_lambda ?? "Unavailable"} />
          </div>
        </div>
      </Card>

      <Card className="p-5">
        <h2 className="text-xl font-semibold">Updates</h2>
        <div className="mt-4 grid gap-4">
          <Metric label="Status" value={updateMessage} />
          {updateNotes ? <Metric label="Release Notes" value={updateNotes} /> : null}
          {updateProgress ? <Metric label="Download" value={formatUpdateProgress(updateProgress)} /> : null}
          <div className="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" disabled={checkingUpdate || installingUpdate} onClick={() => void handleCheckForUpdate()}>
              {checkingUpdate ? "Checking..." : "Check for updates"}
            </Button>
            <Button variant="outline" size="sm" disabled={!pendingUpdate.current || checkingUpdate || installingUpdate} onClick={() => void handleInstallUpdate()}>
              {installingUpdate ? "Installing..." : "Install and relaunch"}
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}

function PreferenceBadges({ empty, label, values }: { empty: string; label: string; values: string[] }) {
  return (
    <div className="grid gap-2">
      <FieldLabel>{label}</FieldLabel>
      <div className="flex flex-wrap gap-2">
        {values.length === 0 ? <span className="text-sm text-muted-foreground">{empty}</span> : null}
        {values.map((value) => <Badge key={value}>{value}</Badge>)}
      </div>
    </div>
  );
}

function formatUpdateProgress(progress: UpdateProgress) {
  const downloaded = formatBytes(progress.downloadedBytes);
  if (progress.totalBytes === null) {
    return downloaded;
  }
  return `${downloaded} / ${formatBytes(progress.totalBytes)}`;
}

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }
  const kib = value / 1024;
  if (kib < 1024) {
    return `${kib.toFixed(1)} KiB`;
  }
  return `${(kib / 1024).toFixed(1)} MiB`;
}
