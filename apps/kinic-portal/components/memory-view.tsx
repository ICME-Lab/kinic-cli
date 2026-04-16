"use client";

// Where: client component for the public memory page.
// What: renders detail cards and drives the public read-only chat request against the Next.js BFF.
// Why: keep the interactive state on the client while server routes remain thin and deterministic.

import { useState, useTransition } from "react";
import {
  buildClaudeCodeMcpCommand,
  buildPublicMemorySearchPrompt,
  buildPublicMemoryShowPrompt,
  type MemoryShowResponse,
} from "@kinic/kinic-share";
import { Check, Copy } from "lucide-react";
import { MemoryStat } from "@/components/memory-stat";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";

type ChatResponse = {
  answer: string;
  context_count: number;
};

type CopyStatusKey = "endpoint" | "claude" | "show" | "search";

export function MemoryView({
  initialMemory,
  mcpEndpoint,
}: {
  initialMemory: MemoryShowResponse;
  mcpEndpoint: string | null;
}) {
  const [query, setQuery] = useState("");
  const [chatAnswer, setChatAnswer] = useState("");
  const [chatContextCount, setChatContextCount] = useState(0);
  const [error, setError] = useState("");
  const [copyStatus, setCopyStatus] = useState<CopyStatusKey | null>(null);
  const [copyError, setCopyError] = useState("");
  const [isPending, startTransition] = useTransition();
  const showPrompt = buildPublicMemoryShowPrompt(initialMemory.memory_id);
  const searchPrompt = buildPublicMemorySearchPrompt(initialMemory.memory_id);
  const claudeCommand = mcpEndpoint ? buildClaudeCodeMcpCommand(mcpEndpoint) : null;

  function submit() {
    startTransition(async () => {
      setError("");
      try {
        const response = await fetch(`/api/memories/${initialMemory.memory_id}/chat`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ query, language: "en" }),
        });
        const payload = parsePayload(await response.json());
        if (!response.ok) {
          throw new Error(payload.error || "request failed");
        }
        setChatAnswer(payload.answer);
        setChatContextCount(payload.context_count);
      } catch (nextError) {
        setError(nextError instanceof Error ? nextError.message : "request failed");
      }
    });
  }

  function copyLabel(key: CopyStatusKey): string {
    return copyStatus === key ? "Copied" : "Copy";
  }

  async function copyText(key: CopyStatusKey, value: string) {
    setCopyError("");
    try {
      await navigator.clipboard.writeText(value);
      setCopyStatus(key);
    } catch {
      setCopyStatus(null);
      setCopyError("Clipboard unavailable");
    }
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="max-w-4xl space-y-6">
          <div className="flex flex-wrap items-center gap-3">
            <Badge variant="outline">Public Memory</Badge>
            <Badge variant="secondary">Read-only</Badge>
            <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
              /m/{initialMemory.memory_id}
            </span>
          </div>
          <div className="space-y-4">
            <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
              Shared Memory Surface
            </p>
            <h1 className="text-[clamp(2.6rem,6vw,4.2rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
              {initialMemory.name}
            </h1>
            <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
              {initialMemory.description || "No description"}
            </p>
          </div>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            <MemoryStat label="Memory ID" value={initialMemory.memory_id} />
            <MemoryStat label="Version" value={initialMemory.version} />
            <MemoryStat label="Dim" value={String(initialMemory.dim)} />
            <MemoryStat label="Owners" value={String(initialMemory.owners.length)} />
          </div>
        </div>
      </section>

      <section className="mt-10 grid gap-5 lg:grid-cols-[minmax(0,1.35fr)_minmax(320px,0.85fr)]">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Ask</Badge>
            <CardTitle>Send a read-only question to the public memory.</CardTitle>
            <CardDescription>Collect relevant context and return only a summarized answer.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-5">
            <Textarea
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Ask this public memory"
              className="min-h-40"
            />
            <div className="flex flex-wrap gap-3">
              <Button disabled={isPending || !query.trim()} onClick={submit}>
                Chat
              </Button>
            </div>
            {error ? (
              <Alert variant="destructive">
                <AlertTitle>Request failed</AlertTitle>
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            ) : null}
            <Separator />
            <div className="rounded-2xl border border-dashed border-border bg-muted/30 px-4 py-4 text-sm leading-7 text-muted-foreground">
              The public UI does not show a raw result list. Internally it uses search to gather
              context and returns only a summarized answer.
            </div>
          </CardContent>
        </Card>

        <div className="grid gap-5">
          <Card>
            <CardHeader className="gap-3">
              <Badge variant="secondary" className="w-fit">Summary</Badge>
              <CardTitle>Read-only Chat</CardTitle>
              <CardDescription>Summarizes {chatContextCount} search results and never writes to the memory.</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="min-h-[280px] whitespace-pre-wrap rounded-2xl border border-border bg-muted/30 p-5 text-sm leading-8 text-foreground">
                {chatAnswer || "No answer yet"}
              </div>
            </CardContent>
          </Card>

          {mcpEndpoint ? (
            <Card className="shadow-none">
              <CardHeader className="gap-3">
                <Badge variant="secondary" className="w-fit">MCP</Badge>
                <CardTitle>Reuse this public memory from an MCP client.</CardTitle>
                <CardDescription>Read-only endpoint, Claude Code command, and copyable prompts for this memory.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4 text-sm leading-7 text-muted-foreground">
                <div className="rounded-2xl border border-border bg-muted/20 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Endpoint</p>
                      <p className="mt-1 text-sm text-foreground">Use this URL in any generic MCP client.</p>
                    </div>
                    <CopyButton onClick={() => copyText("endpoint", mcpEndpoint)} copied={copyStatus === "endpoint"}>
                      {copyLabel("endpoint")}
                    </CopyButton>
                  </div>
                  <code className="mt-3 block break-all rounded-2xl border border-border bg-background px-4 py-3 font-mono text-[12px] text-foreground">
                    {mcpEndpoint}
                  </code>
                </div>

                {claudeCommand ? (
                  <div className="rounded-2xl border border-border bg-muted/20 p-4">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Claude Code</p>
                        <p className="mt-1 text-sm text-foreground">Add the remote MCP server with one command.</p>
                      </div>
                      <CopyButton onClick={() => copyText("claude", claudeCommand)} copied={copyStatus === "claude"}>
                        {copyLabel("claude")}
                      </CopyButton>
                    </div>
                    <code className="mt-3 block break-all rounded-2xl border border-border bg-background px-4 py-3 font-mono text-[12px] text-foreground">
                      {claudeCommand}
                    </code>
                  </div>
                ) : null}

                <div className="rounded-2xl border border-border bg-muted/20 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Example Prompts</p>
                      <p className="mt-1 text-sm text-foreground">Prompt an agent to inspect or search this memory by id.</p>
                    </div>
                  </div>
                  <div className="mt-3 space-y-3">
                    <PromptRow
                      prompt={showPrompt}
                      copied={copyStatus === "show"}
                      onCopy={() => copyText("show", showPrompt)}
                    />
                    <PromptRow
                      prompt={searchPrompt}
                      copied={copyStatus === "search"}
                      onCopy={() => copyText("search", searchPrompt)}
                    />
                  </div>
                </div>

                <div className="rounded-2xl border border-dashed border-border bg-background px-4 py-4">
                  <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Contract</p>
                  <p className="mt-2 text-sm text-foreground">The shared surface and MCP endpoint stay anonymous and read-only. Owner authentication and write permissions do not belong here.</p>
                </div>
                {copyError ? <p className="text-sm text-muted-foreground">{copyError}</p> : null}
              </CardContent>
            </Card>
          ) : null}
        </div>
      </section>
    </main>
  );
}

function PromptRow({
  prompt,
  copied,
  onCopy,
}: {
  prompt: string;
  copied: boolean;
  onCopy: () => void;
}) {
  return (
    <div className="rounded-2xl border border-border bg-background p-3">
      <div className="flex items-start justify-between gap-3">
        <code className="block break-all font-mono text-[12px] leading-6 text-foreground">{prompt}</code>
        <CopyButton onClick={onCopy} copied={copied}>
          {copied ? "Copied" : "Copy"}
        </CopyButton>
      </div>
    </div>
  );
}

function CopyButton({
  children,
  copied,
  onClick,
}: {
  children: string;
  copied: boolean;
  onClick: () => void;
}) {
  return (
    <Button variant="outline" size="sm" onClick={onClick} className="shrink-0">
      {copied ? <Check className="size-4" /> : <Copy className="size-4" />}
      {children}
    </Button>
  );
}

function parsePayload(value: unknown): ChatResponse & { error?: string } {
  const record = toRecord(value);
  if (!record) {
    return { answer: "", context_count: 0, error: "invalid response" };
  }
  return {
    answer: typeof record.answer === "string" ? record.answer : "",
    context_count: typeof record.context_count === "number" ? record.context_count : 0,
    error: typeof record.error === "string" ? record.error : undefined,
  };
}

function toRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
