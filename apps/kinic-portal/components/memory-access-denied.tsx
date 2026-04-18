// Where: server-rendered fallback for public memory pages.
// What: explains that anonymous users cannot read the target memory canister.
// Why: 403 should be explicit instead of collapsing into a generic not-found page.

import { AlertCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function MemoryAccessDenied({ memoryId }: { memoryId?: string }) {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="max-w-4xl space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <Badge variant="secondary">Access Denied</Badge>
            <Badge variant="outline">Anonymous</Badge>
          </div>
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Public Memory Access
          </p>
          <h1 className="text-[clamp(2.6rem,6vw,4.2rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
            Anonymous access is blocked.
          </h1>
          <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
            Anonymous reads are denied for this memory canister.
          </p>
        </div>
      </section>

      <section className="mt-10 grid gap-5 md:grid-cols-[minmax(0,1.15fr)_minmax(280px,0.85fr)]">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Advisory</Badge>
            <CardTitle className="flex items-center gap-3">
              <AlertCircle className="size-5" />
              Access required
            </CardTitle>
            <CardDescription>Ask the owner to grant anonymous read access.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm text-muted-foreground">
            {memoryId ? <p className="font-mono text-foreground/80">{memoryId}</p> : null}
            <p>The shared URL can exist, but the public page does not render when anonymous `get_name()` fails.</p>
          </CardContent>
        </Card>

        <Card className="shadow-none">
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Resolution</Badge>
            <CardTitle>What to do next</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm leading-7 text-muted-foreground">
            <p>Ask the owner to grant the `anonymous reader` role.</p>
            <p>Once the public condition is satisfied, the same URL will expose details and read-only chat.</p>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
