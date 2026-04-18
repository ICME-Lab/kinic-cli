// Where: Next.js home page for the Kinic portal.
// What: explains the v1 public-sharing workflow and points users at the dynamic memory route.
// Why: initial release has no share creation UI, so the landing page must set expectations clearly.

import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";

const EXAMPLE_MEMORY_ID = "ywega-gaaaa-aaaak-apg6q-cai";

export default function HomePage() {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="mx-auto flex max-w-4xl flex-col items-center gap-6 text-center">
          <div className="space-y-4">
            <h1 className="text-[clamp(2.8rem,7vw,4.5rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
              Share a public memory.
            </h1>
            <p className="mx-auto max-w-2xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
              Browse, search, and ask in one page.
            </p>
          </div>
          <div className="flex flex-wrap items-center justify-center gap-3">
            <Link
              href={`/m/${EXAMPLE_MEMORY_ID}`}
              className={buttonVariants({ className: "min-w-36 !text-background hover:!text-background" })}
            >
              Example Memory
            </Link>
            <Link
              href="https://github.com/ICME-Lab/kinic-cli"
              className={cn(buttonVariants({ variant: "secondary" }), "min-w-40")}
            >
              Publish from CLI
            </Link>
          </div>
          <Card className="w-full max-w-xl rounded-full bg-background/90 shadow-none">
            <CardContent className="flex flex-wrap items-center justify-center gap-3 px-5 py-3">
              <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Example URL</span>
              <Separator orientation="vertical" className="hidden h-4 sm:block" />
              <code className="font-mono text-sm text-foreground">/m/{EXAMPLE_MEMORY_ID}</code>
            </CardContent>
          </Card>
        </div>
      </section>

      <section className="mt-10">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Built For Sharing</Badge>
            <CardTitle>Built for public memory sharing.</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 text-sm leading-7 text-muted-foreground">
            <p>Share one link to let anyone browse, search, and ask your public memory.</p>
            <div className="grid gap-3 md:grid-cols-3">
              <div className="rounded-2xl border border-border bg-muted px-4 py-4">
                <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Share</p>
                <p className="mt-2 text-sm text-foreground">Turn a public memory into a page you can send anywhere.</p>
              </div>
              <div className="rounded-2xl border border-border bg-muted px-4 py-4">
                <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Search</p>
                <p className="mt-2 text-sm text-foreground">Find the right part fast with built-in memory search.</p>
              </div>
              <div className="rounded-2xl border border-border bg-muted px-4 py-4">
                <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">Ask</p>
                <p className="mt-2 text-sm text-foreground">Get concise answers grounded in the memory itself.</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="mt-5">
        <p className="text-center text-sm text-muted-foreground">Read-only. Public memories only.</p>
      </section>
    </main>
  );
}
