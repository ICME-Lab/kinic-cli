"use client";

// Where: client fallback for public memory shell fetch failures that resolve to 404.
// What: explains that the target shared memory does not exist or is no longer public.
// Why: the portal page no longer returns server-side 404 once the shell is static.

import { SearchX } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function MemoryNotFound({ memoryId }: { memoryId?: string }) {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="max-w-4xl space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <Badge variant="secondary">Not Found</Badge>
            <Badge variant="outline">Public Memory</Badge>
          </div>
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Public Memory Access
          </p>
          <h1 className="text-[clamp(2.6rem,6vw,4.2rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
            Shared memory not found.
          </h1>
          <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
            The target memory does not exist or is no longer available for public reading.
          </p>
        </div>
      </section>

      <section className="mt-10 grid gap-5 md:grid-cols-[minmax(0,1.15fr)_minmax(280px,0.85fr)]">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Status</Badge>
            <CardTitle className="flex items-center gap-3">
              <SearchX className="size-5" />
              Memory unavailable
            </CardTitle>
            <CardDescription>The shared URL no longer resolves to a public memory canister.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm text-muted-foreground">
            {memoryId ? <p className="font-mono text-foreground/80">{memoryId}</p> : null}
            <p>Verify the canister id and confirm the memory is still deployed on the selected IC network.</p>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
