// Where: Insert workflow.
// What: supports file and inline text insert for MVP.
// Why: keep content ingestion available without recreating CLI flags.

import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Button, Card, FieldLabel, Input, Textarea, Badge } from "../primitives";
import { useDesktopStore } from "@/store/useDesktopStore";

export function InsertView() {
  const mode = useDesktopStore((state) => state.insertMode);
  const memoryId = useDesktopStore((state) => state.insertMemoryId);
  const tag = useDesktopStore((state) => state.insertTag);
  const text = useDesktopStore((state) => state.insertText);
  const filePath = useDesktopStore((state) => state.insertFilePath);
  const loading = useDesktopStore((state) => state.loading);
  const setMode = useDesktopStore((state) => state.setInsertMode);
  const setMemoryId = useDesktopStore((state) => state.setInsertMemoryId);
  const setTag = useDesktopStore((state) => state.setInsertTag);
  const setText = useDesktopStore((state) => state.setInsertText);
  const setFilePath = useDesktopStore((state) => state.setInsertFilePath);
  const submitInsert = useDesktopStore((state) => state.submitInsert);

  async function chooseFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Memory content", extensions: ["md", "markdown", "mdx", "txt", "json", "yaml", "yml", "csv", "log", "pdf"] }],
    });
    if (typeof selected === "string") {
      setFilePath(selected);
    }
  }

  return (
    <Card className="max-w-4xl p-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold">Insert Content</h2>
          <p className="mt-1 text-sm text-muted-foreground">Add file or inline text to a selected memory.</p>
        </div>
        <Badge>{mode}</Badge>
      </div>

      <div className="mt-6 grid gap-5">
        <div className="grid grid-cols-3 gap-2 rounded-full border border-border bg-muted p-1">
          <ModeButton active={mode === "file"} label="File" onClick={() => setMode("file")} />
          <ModeButton active={mode === "inline_text"} label="Inline Text" onClick={() => setMode("inline_text")} />
          <ModeButton active={mode === "manual_embedding"} disabled label="Manual Embedding" onClick={() => setMode("manual_embedding")} />
        </div>

        <div className="grid gap-2">
          <FieldLabel htmlFor="insert-memory">Memory ID</FieldLabel>
          <Input id="insert-memory" value={memoryId} onChange={(event) => setMemoryId(event.target.value)} />
        </div>

        {mode === "file" ? (
          <>
            <div className="grid gap-2">
              <FieldLabel htmlFor="insert-file">File Path</FieldLabel>
              <div className="grid grid-cols-[1fr_auto] gap-2">
                <Input id="insert-file" value={filePath} onChange={(event) => setFilePath(event.target.value)} />
                <Button variant="secondary" onClick={() => void chooseFile()}>
                  <FolderOpen className="size-4" />
                  Choose
                </Button>
              </div>
            </div>
            <div className="grid gap-2">
              <FieldLabel htmlFor="insert-tag">Tag Override</FieldLabel>
              <Input id="insert-tag" value={tag} onChange={(event) => setTag(event.target.value)} placeholder="Auto from file name" />
            </div>
          </>
        ) : null}

        {mode === "inline_text" ? (
          <>
            <div className="grid gap-2">
              <FieldLabel htmlFor="inline-tag">Tag</FieldLabel>
              <Input id="inline-tag" value={tag} onChange={(event) => setTag(event.target.value)} />
            </div>
            <div className="grid gap-2">
              <FieldLabel htmlFor="inline-text">Text</FieldLabel>
              <Textarea id="inline-text" value={text} onChange={(event) => setText(event.target.value)} />
            </div>
          </>
        ) : null}

        {mode === "manual_embedding" ? (
          <div className="rounded-2xl border border-border bg-muted px-4 py-5 text-sm text-muted-foreground">
            Manual embedding is planned after the desktop MVP shell is stable.
          </div>
        ) : null}

        <Button disabled={loading || mode === "manual_embedding"} onClick={() => void submitInsert()}>
          {loading ? "Submitting..." : "Submit Insert"}
        </Button>
      </div>
    </Card>
  );
}

function ModeButton({ active, disabled, label, onClick }: { active: boolean; disabled?: boolean; label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={active ? activeMode : inactiveMode}
    >
      {label}
    </button>
  );
}

const activeMode = "rounded-full bg-background px-4 py-2 text-sm font-medium text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] disabled:opacity-50";
const inactiveMode = "rounded-full px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground disabled:opacity-50";
