// Where: Memories workflow.
// What: shows memory list, search, results, and selected memory details.
// Why: this is the primary desktop operating surface.

import { Search } from "lucide-react";
import { Button, Card, FieldLabel, Input, Metric, Badge } from "../primitives";
import { shortPrincipal, statusText, valueOrUnavailable } from "@/desktop/format";
import { cn } from "@/lib/utils";
import { useDesktopStore } from "@/store/useDesktopStore";

export function MemoriesView() {
  const memories = useDesktopStore((state) => state.memories);
  const selectedMemoryId = useDesktopStore((state) => state.selectedMemoryId);
  const details = useDesktopStore((state) => state.selectedDetails);
  const query = useDesktopStore((state) => state.searchQuery);
  const scope = useDesktopStore((state) => state.searchScope);
  const results = useDesktopStore((state) => state.searchResults);
  const loading = useDesktopStore((state) => state.loading);
  const error = useDesktopStore((state) => state.error);
  const setQuery = useDesktopStore((state) => state.setSearchQuery);
  const setScope = useDesktopStore((state) => state.setSearchScope);
  const selectMemory = useDesktopStore((state) => state.selectMemory);
  const submitSearch = useDesktopStore((state) => state.submitSearch);

  return (
    <div className="grid gap-5 xl:grid-cols-[320px_minmax(0,1fr)]">
      <Card className="overflow-hidden">
        <div className="border-b border-border px-5 py-4">
          <h2 className="text-lg font-semibold">Memories</h2>
          <p className="text-sm text-muted-foreground">{statusText(error, loading, `${memories.length} records`)}</p>
        </div>
        <div className="max-h-[680px] overflow-auto p-2">
          {memories.map((memory) => {
            const memoryId = memory.searchable_memory_id ?? memory.id;
            return (
              <button
                key={memory.id}
                type="button"
                onClick={() => void selectMemory(memoryId)}
                className={cn(
                  "mb-1 w-full rounded-xl border px-3 py-3 text-left transition-colors",
                  selectedMemoryId === memoryId
                    ? "border-accent/30 bg-accent/10"
                    : "border-transparent hover:border-border hover:bg-muted/70",
                )}
              >
                <div className="text-sm font-medium">{memory.name}</div>
                <div className="mt-1 truncate font-mono text-[11px] text-muted-foreground">{memoryId}</div>
                <div className="mt-2 flex gap-2">
                  <Badge>{memory.status}</Badge>
                  {memory.dim ? <Badge>{memory.dim} dim</Badge> : null}
                </div>
              </button>
            );
          })}
        </div>
      </Card>

      <div className="grid gap-5">
        <Card className="p-5">
          <div className="grid gap-4 lg:grid-cols-[1fr_auto]">
            <div className="grid gap-2">
              <FieldLabel htmlFor="memory-search">Search</FieldLabel>
              <Input
                id="memory-search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search memory contents"
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    void submitSearch();
                  }
                }}
              />
            </div>
            <div className="flex items-end gap-2">
              <button type="button" onClick={() => setScope("all")} className={scope === "all" ? activeSegment : inactiveSegment}>
                All memories
              </button>
              <button type="button" onClick={() => setScope("selected")} className={scope === "selected" ? activeSegment : inactiveSegment}>
                Selected
              </button>
              <Button disabled={loading || !query.trim()} onClick={() => void submitSearch()}>
                <Search className="size-4" />
                Search
              </Button>
            </div>
          </div>
        </Card>

        <Card className="p-5">
          <h2 className="text-lg font-semibold">{details?.display_name ?? "Select a memory"}</h2>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            {details?.metadata_name || "Memory details appear here after selection."}
          </p>
          <div className="mt-5 grid gap-3 md:grid-cols-3">
            <Metric label="Memory ID" value={shortPrincipal(details?.memory_id)} />
            <Metric label="Version" value={valueOrUnavailable(details?.version)} />
            <Metric label="Dim" value={valueOrUnavailable(details?.dim)} />
            <Metric label="Stable Memory" value={valueOrUnavailable(details?.stable_memory_size)} />
            <Metric label="Cycles" value={valueOrUnavailable(details?.cycle_amount)} />
            <Metric label="Users" value={details?.users.length ?? 0} />
          </div>
        </Card>

        {results.length > 0 ? (
          <Card className="p-5">
            <h2 className="text-lg font-semibold">Search Results</h2>
            <div className="mt-4 grid gap-3">
              {results.map((result, index) => (
                <div key={`${result.memory_id}-${index}`} className="rounded-2xl border border-border bg-muted/40 px-4 py-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge>{result.score.toFixed(3)}</Badge>
                    {result.tag ? <Badge>{result.tag}</Badge> : null}
                    <span className="font-mono text-[11px] text-muted-foreground">{shortPrincipal(result.memory_id)}</span>
                  </div>
                  <p className="mt-2 text-sm leading-6">{result.sentence}</p>
                </div>
              ))}
            </div>
          </Card>
        ) : null}
      </div>
    </div>
  );
}

const activeSegment = "h-11 rounded-full border border-border bg-background px-4 text-sm font-medium shadow-[0_1px_2px_rgba(0,0,0,0.04)]";
const inactiveSegment = "h-11 rounded-full border border-border bg-muted px-4 text-sm font-medium text-muted-foreground hover:text-foreground";
