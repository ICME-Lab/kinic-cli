"use client";

// Where: client component for the public memory page.
// What: renders detail cards and drives the public read-only chat request against the Next.js BFF.
// Why: keep the interactive state on the client while server routes remain thin and deterministic.

import { useEffect, useState, useTransition } from "react";
import {
  buildChatGptMemoryPrompt,
  buildChatGptPromptUrl,
  type MemoryShowResponse,
} from "@kinic/kinic-share";
import { Check, Copy } from "lucide-react";
import { FaDiscord, FaLinkedinIn, FaTelegram, FaXTwitter } from "react-icons/fa6";
import { SiOpenai } from "react-icons/si";
import { MemoryStat } from "@/components/memory-stat";
import { MemorySummary } from "@/components/memory-summary";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

type ChatResponse = {
  answer: string;
  context_count: number;
};

type CopyStatusKey = "share" | "discord" | "chatgpt";

export function MemoryView({
  initialMemory,
  mcpEndpoint,
}: {
  initialMemory: MemoryShowResponse;
  mcpEndpoint: string | null;
}) {
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [answer, setAnswer] = useState("");
  const [contextCount, setContextCount] = useState(0);
  const [copyStatus, setCopyStatus] = useState<CopyStatusKey | null>(null);
  const [copyError, setCopyError] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [isPending, startTransition] = useTransition();
  const chatGptPrompt = buildChatGptMemoryPrompt(initialMemory.memory_id);
  const chatGptUrl = buildChatGptPromptUrl(chatGptPrompt);
  const shareLinks = buildShareLinks(currentUrl, initialMemory.name, initialMemory.description);

  useEffect(() => {
    setCurrentUrl(window.location.href);
  }, []);

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
        setAnswer(payload.answer);
        setContextCount(payload.context_count);
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

  function openInChatGpt() {
    void copyText("chatgpt", chatGptPrompt);
    window.open(chatGptUrl, "_blank", "noopener,noreferrer");
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-16 pt-4 md:px-6 md:pb-20">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-7 md:px-10 md:py-10">
        <div className="space-y-6">
          <div className="space-y-4">
            <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
              Memory Name
            </p>
            <h1 className="text-[clamp(2.6rem,6vw,4.2rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
              {initialMemory.name}
            </h1>
            <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
              {initialMemory.description || "No description"}
            </p>
            <div className="grid w-full gap-4">
              <MemorySummary memoryId={initialMemory.memory_id} />
              <div className="grid gap-3 md:grid-cols-3">
                <MemoryStat label="Memory ID" value={initialMemory.memory_id} />
                <MemoryStat label="Version" value={initialMemory.version} />
                <MemoryStat label="Dim" value={String(initialMemory.dim)} />
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="mt-10 grid gap-5 lg:grid-cols-[minmax(0,1.35fr)_minmax(320px,0.85fr)]">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Ask</Badge>
            <CardTitle>Send a question to the memory.</CardTitle>
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
            {answer ? (
              <>
                <Separator />
                <div className="space-y-3 rounded-2xl border border-border bg-muted/20 px-4 py-4">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">Answer</Badge>
                    <span className="text-sm text-muted-foreground">
                      Grounded in {contextCount} search result{contextCount === 1 ? "" : "s"}.
                    </span>
                  </div>
                  <p className="text-sm leading-7 text-foreground">{answer}</p>
                </div>
              </>
            ) : null}
          </CardContent>
        </Card>

        <div className="grid gap-5">
          <Card className="shadow-none">
            <CardHeader className="gap-3">
              <Badge variant="secondary" className="w-fit">Share</Badge>
              <CardTitle className="font-normal">Share this memory anywhere.</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-3">
                <ShareLink href={shareLinks.x} label="Share on X" className="!text-zinc-900 hover:!text-foreground active:!text-foreground focus-visible:!text-foreground">
                  <FaXTwitter className="size-4" />
                </ShareLink>
                <ShareLink href={shareLinks.linkedin} label="Share on LinkedIn" className="!text-sky-700 hover:!text-foreground active:!text-foreground focus-visible:!text-foreground">
                  <FaLinkedinIn className="size-4" />
                </ShareLink>
                <ShareLink href={shareLinks.telegram} label="Share on Telegram" className="!text-sky-500 hover:!text-foreground active:!text-foreground focus-visible:!text-foreground">
                  <FaTelegram className="size-4" />
                </ShareLink>
                <button
                  type="button"
                  aria-label="Copy share URL for Discord"
                  onClick={() => copyText("discord", currentUrl || `/m/${initialMemory.memory_id}`)}
                  className="inline-flex size-9 items-center justify-center rounded-full border border-border bg-background text-indigo-500 shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted hover:!text-foreground active:!text-foreground focus-visible:!text-foreground"
                >
                  {copyStatus === "discord" ? <Check className="size-4" /> : <FaDiscord className="size-4" />}
                </button>
                <ShareIconButton
                  copied={copyStatus === "share"}
                  label="Copy share URL"
                  onClick={() => copyText("share", currentUrl || `/m/${initialMemory.memory_id}`)}
                />
              </div>
            </CardContent>
          </Card>

          {mcpEndpoint ? (
            <Card className="shadow-none">
              <CardHeader className="gap-3">
                <Badge variant="secondary" className="w-fit">ChatGPT</Badge>
                <CardTitle className="font-normal">Use this memory in ChatGPT.</CardTitle>
              </CardHeader>
              <CardContent className="text-sm leading-7 text-muted-foreground">
                <div className="flex items-center gap-3">
                  <button
                    type="button"
                    aria-label="Open in ChatGPT"
                    onClick={openInChatGpt}
                    className="inline-flex size-9 shrink-0 items-center justify-center rounded-full border border-border bg-background text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted"
                  >
                    <SiOpenai className="size-5" />
                  </button>
                  <p className="text-sm leading-6 text-foreground">
                    Requires the Kinic app in ChatGPT.
                  </p>
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

function ShareLink({
  className,
  children,
  href,
  label,
}: {
  className?: string;
  children: React.ReactNode;
  href: string;
  label: string;
}) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      aria-label={label}
      className={cn(
        "inline-flex size-9 items-center justify-center rounded-full border border-border bg-background text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted",
        className,
      )}
    >
      {children}
    </a>
  );
}

function ShareIconButton({
  copied,
  label,
  onClick,
}: {
  copied: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className="inline-flex size-9 items-center justify-center rounded-full border border-border bg-background text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted"
    >
      {copied ? <Check className="size-4" /> : <Copy className="size-4" />}
    </button>
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

function buildShareLinks(url: string, title: string, description: string | null) {
  const encodedUrl = encodeURIComponent(url);
  const shareText = buildXShareText(title, description);
  const encodedTitle = encodeURIComponent(shareText);
  return {
    x: `https://twitter.com/intent/tweet?url=${encodedUrl}&text=${encodedTitle}`,
    linkedin: `https://www.linkedin.com/sharing/share-offsite/?url=${encodedUrl}`,
    telegram: `https://t.me/share/url?url=${encodedUrl}&text=${encodedTitle}`,
  };
}

function buildXShareText(title: string, description: string | null): string {
  const normalizedTitle = title.trim();
  const normalizedDescription = description?.trim();
  const headline = "Explore this public memory on Kinic.";
  return [headline, normalizedTitle, normalizedDescription].filter(Boolean).join("\n\n");
}
