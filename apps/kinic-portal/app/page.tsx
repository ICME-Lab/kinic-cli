// Where: Next.js home page for the Kinic portal.
// What: explains the v1 public-sharing workflow and points users at the dynamic memory route.
// Why: initial release has no share creation UI, so the landing page must set expectations clearly.

import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";

export default function HomePage() {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="mx-auto flex max-w-4xl flex-col items-center gap-6 text-center">
          <Badge variant="outline">Kinic Portal</Badge>
          <div className="space-y-4">
            <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
              Public Memory Documentation Surface
            </p>
            <h1 className="text-[clamp(2.8rem,7vw,4.5rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
              Share a public read-only memory through one URL.
            </h1>
            <p className="mx-auto max-w-2xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
              v1 exposes any memory canister that already grants the `anonymous reader` role at
              `/m/[memoryId]`. The web surface does not create sharing settings. Publish the memory
              from the CLI or TUI first, then distribute the URL.
            </p>
          </div>
          <div className="flex flex-wrap items-center justify-center gap-3">
            <Link href="/m/aaaaa-aa" className={buttonVariants({ className: "min-w-36" })}>
              Example Memory
            </Link>
            <Link
              href="https://github.com/kinic-labs/kinic"
              className={cn(buttonVariants({ variant: "secondary" }), "min-w-40")}
            >
              CLI / TUI Workflow
            </Link>
          </div>
          <Card className="w-full max-w-xl rounded-full bg-background/90 shadow-none">
            <CardContent className="flex flex-wrap items-center justify-center gap-3 px-5 py-3">
              <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Example URL</span>
              <Separator orientation="vertical" className="hidden h-4 sm:block" />
              <code className="font-mono text-sm text-foreground">/m/aaaaa-aa</code>
            </CardContent>
          </Card>
        </div>
      </section>

      <section className="mt-12 grid gap-5 md:grid-cols-[1.3fr_0.7fr]">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Includes</Badge>
            <CardTitle>The public surface stays focused on documentation as product.</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 text-sm leading-7 text-muted-foreground">
            <p>One URL combines memory details, search, and summarized read-only Q&amp;A.</p>
            <div className="grid gap-3 md:grid-cols-3">
              <div className="rounded-2xl border border-border bg-muted px-4 py-4">
                <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Detail</p>
                <p className="mt-2 text-sm text-foreground">Memory name, description, and technical summary.</p>
              </div>
              <div className="rounded-2xl border border-border bg-muted px-4 py-4">
                <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Search</p>
                <p className="mt-2 text-sm text-foreground">Read-only queries and ranked result snippets.</p>
              </div>
              <div className="rounded-2xl border border-border bg-muted px-4 py-4">
                <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Q&amp;A</p>
                <p className="mt-2 text-sm text-foreground">A chat surface that returns only summarized answers.</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Out of Scope</Badge>
            <CardTitle>Share creation UI is still out of scope.</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 text-sm leading-7 text-muted-foreground">
            <p>
              No owner authentication, write actions, or ChatGPT app UI lives here. Public visibility
              is determined only by the canister&apos;s `anonymous reader` setting.
            </p>
            <div className="rounded-2xl border border-dashed border-border bg-background px-4 py-4">
              <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Current boundary</p>
              <p className="mt-2 text-sm text-foreground">Publish in the CLI or TUI, then distribute and browse in the portal.</p>
            </div>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
