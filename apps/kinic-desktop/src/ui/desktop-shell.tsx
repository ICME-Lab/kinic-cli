// Where: authenticated desktop app shell.
// What: renders navigation, toolbar, active workflow, and transient status.
// Why: keep desktop workflows dense and always accessible.

import { Database, FilePlus2, List, PlusCircle, Settings } from "lucide-react";
import type { ComponentType } from "react";
import { Button, Badge } from "./primitives";
import { CreateView } from "./views/create-view";
import { InsertView } from "./views/insert-view";
import { MemoriesView } from "./views/memories-view";
import { SettingsView } from "./views/settings-view";
import { cn } from "@/lib/utils";
import { useDesktopStore, type ActiveTab } from "@/store/useDesktopStore";

const tabs: Array<{ id: ActiveTab; label: string; icon: ComponentType<{ className?: string }> }> = [
  { id: "memories", label: "Memories", icon: List },
  { id: "insert", label: "Insert", icon: FilePlus2 },
  { id: "create", label: "Create", icon: PlusCircle },
  { id: "settings", label: "Settings", icon: Settings },
];

export function DesktopShell() {
  const activeTab = useDesktopStore((state) => state.activeTab);
  const setActiveTab = useDesktopStore((state) => state.setActiveTab);
  const session = useDesktopStore((state) => state.session);
  const loading = useDesktopStore((state) => state.loading);
  const error = useDesktopStore((state) => state.error);
  const toast = useDesktopStore((state) => state.toast);
  const clearToast = useDesktopStore((state) => state.clearToast);
  const refreshMemories = useDesktopStore((state) => state.refreshMemories);

  return (
    <div className="grid min-h-screen grid-cols-[240px_minmax(0,1fr)] bg-background">
      <aside className="border-r border-border px-4 py-5">
        <div className="mb-6 flex items-center gap-3 px-2">
          <div className="inline-flex size-9 items-center justify-center rounded-full border border-border bg-muted">
            <Database className="size-4" />
          </div>
          <div>
            <div className="text-sm font-semibold">Kinic Memory</div>
            <div className="text-xs text-muted-foreground">{session?.network}</div>
          </div>
        </div>
        <nav className="grid gap-1">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                className={cn(
                  "flex items-center gap-3 rounded-xl border px-3 py-2 text-sm transition-colors",
                  activeTab === tab.id
                    ? "border-border bg-muted text-foreground"
                    : "border-transparent text-muted-foreground hover:border-border hover:bg-muted/60 hover:text-foreground",
                )}
              >
                <Icon className="size-4" />
                {tab.label}
              </button>
            );
          })}
        </nav>
      </aside>
      <main className="min-w-0">
        <header className="flex h-16 items-center justify-between border-b border-border px-6">
          <div className="flex items-center gap-3">
            <Badge>{session?.identity}</Badge>
            <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
              {session?.principal_id}
            </span>
          </div>
          <Button variant="secondary" size="sm" disabled={loading} onClick={() => void refreshMemories()}>
            Refresh
          </Button>
        </header>
        <section className="p-6">
          {activeTab === "memories" ? <MemoriesView /> : null}
          {activeTab === "insert" ? <InsertView /> : null}
          {activeTab === "create" ? <CreateView /> : null}
          {activeTab === "settings" ? <SettingsView /> : null}
        </section>
      </main>
      {error ? <StatusBar tone="error" message={error} onClose={clearToast} /> : null}
      {toast && !error ? <StatusBar tone="ok" message={toast} onClose={clearToast} /> : null}
    </div>
  );
}

function StatusBar({ tone, message, onClose }: { tone: "ok" | "error"; message: string; onClose: () => void }) {
  return (
    <button
      type="button"
      onClick={onClose}
      className={cn(
        "fixed bottom-5 right-5 max-w-lg rounded-2xl border px-4 py-3 text-left text-sm shadow-[0_8px_24px_rgba(0,0,0,0.08)]",
        tone === "ok" ? "border-border bg-background text-foreground" : "border-red-200 bg-red-50 text-red-700",
      )}
    >
      {message}
    </button>
  );
}
