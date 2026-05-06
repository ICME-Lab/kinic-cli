// Where: interactive memory detail view for the public route.
// What: renders one public memory and drives read-only requests against the dedicated public API Worker.
// Why: portal pages stay static while interactive API work moves off the Next server path.

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
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { buildPublicApiUrl } from "@/lib/public-api";
import { cn } from "@/lib/utils";
import { DEV_VITE_SHELL_CHAT_ERROR, hasInjectedRuntimeConfig } from "@/src/runtime-config";

type ChatResponse = {
  answer: string;
  context_count: number;
};

type CopyStatusKey = "share" | "discord" | "chatgpt";

export function MemoryView({
  memory,
  mcpEndpoint,
  publicApiOrigin,
}: {
  memory: MemoryShowResponse;
  mcpEndpoint: string | null;
  publicApiOrigin: string;
}) {
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [answer, setAnswer] = useState("");
  const [contextCount, setContextCount] = useState(0);
  const [copyStatus, setCopyStatus] = useState<CopyStatusKey | null>(null);
  const [copyError, setCopyError] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [language, setLanguage] = useState("en");
  const [isPending, startTransition] = useTransition();
  const chatGptPrompt = buildChatGptMemoryPrompt(memory.memory_id);
  const chatGptUrl = buildChatGptPromptUrl(chatGptPrompt);
  const shareLinks = buildShareLinks(currentUrl, memory.name, memory.description);
  const isShareReady = currentUrl.length > 0;

  useEffect(() => {
    setCurrentUrl(window.location.href);
    setLanguage(window.navigator.language || "en");
  }, []);

  function submit() {
    startTransition(async () => {
      setError("");
      setAnswer("");
      setContextCount(0);
      try {
        if (!hasInjectedRuntimeConfig() && publicApiOrigin === window.location.origin) {
          throw new Error(DEV_VITE_SHELL_CHAT_ERROR);
        }
        const response = await fetch(buildPublicApiUrl(`/api/public/memories/${memory.memory_id}/chat`, publicApiOrigin), {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ query, language }),
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
              {memory.name}
            </h1>
            <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
              {memory.description || "No description"}
            </p>
            <div className="grid w-full gap-4">
              <MemorySummary memoryId={memory.memory_id} />
              <div className="grid gap-3 md:grid-cols-3">
                <MemoryStat label="Memory ID" value={memory.memory_id} />
                <MemoryStat label="Version" value={memory.version} />
                <MemoryStat label="Dim" value={String(memory.dim)} />
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
                <ShareLink disabled={!isShareReady} href={shareLinks.x} label="Share on X" className="!text-zinc-900 hover:!text-foreground active:!text-foreground focus-visible:!text-foreground">
                  <FaXTwitter className="size-4" />
                </ShareLink>
                <ShareLink disabled={!isShareReady} href={shareLinks.linkedin} label="Share on LinkedIn" className="!text-sky-700 hover:!text-foreground active:!text-foreground focus-visible:!text-foreground">
                  <FaLinkedinIn className="size-4" />
                </ShareLink>
                <ShareLink disabled={!isShareReady} href={shareLinks.telegram} label="Share on Telegram" className="!text-sky-500 hover:!text-foreground active:!text-foreground focus-visible:!text-foreground">
                  <FaTelegram className="size-4" />
                </ShareLink>
                <button
                  type="button"
                  aria-label="Copy share URL for Discord"
                  disabled={!isShareReady}
                  onClick={() => copyText("discord", currentUrl)}
                  className="inline-flex size-9 items-center justify-center rounded-full border border-border bg-background text-indigo-500 shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted hover:!text-foreground active:!text-foreground focus-visible:!text-foreground disabled:pointer-events-none disabled:opacity-50"
                >
                  {copyStatus === "discord" ? <Check className="size-4" /> : <FaDiscord className="size-4" />}
                </button>
                <ShareIconButton
                  copied={copyStatus === "share"}
                  disabled={!isShareReady}
                  label="Copy share URL"
                  onClick={() => copyText("share", currentUrl)}
                />
              </div>
              {copyError ? <p className="text-sm text-muted-foreground">{copyError}</p> : null}
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
  disabled,
  href,
  label,
}: {
  className?: string;
  children: React.ReactNode;
  disabled: boolean;
  href: string;
  label: string;
}) {
  const classes = cn(
    "inline-flex size-9 items-center justify-center rounded-full border border-border bg-background text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted disabled:pointer-events-none disabled:opacity-50",
    className,
  );
  if (disabled) {
    return (
      <button type="button" disabled aria-label={label} className={classes}>
        {children}
      </button>
    );
  }
  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      aria-label={label}
      className={classes}
    >
      {children}
    </a>
  );
}

function ShareIconButton({
  copied,
  disabled,
  label,
  onClick,
}: {
  copied: boolean;
  disabled: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className="inline-flex size-9 items-center justify-center rounded-full border border-border bg-background text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-input hover:bg-muted disabled:pointer-events-none disabled:opacity-50"
    >
      {copied ? <Check className="size-4" /> : <Copy className="size-4" />}
    </button>
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
