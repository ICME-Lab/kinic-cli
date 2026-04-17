// Where: server-rendered fallback for transient public memory verification failures.
// What: explains that the memory could not be verified right now and suggests retrying.
// Why: query certificate failures should not crash the page or masquerade as ACL denial.

import { WifiOff } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function MemoryTemporaryError({ memoryId }: { memoryId?: string }) {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="max-w-4xl space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <Badge variant="secondary">Temporary Error</Badge>
            <Badge variant="outline">Retry</Badge>
          </div>
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Public Memory Access
          </p>
          <h1 className="text-[clamp(2.6rem,6vw,4.2rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
            Temporary network error
          </h1>
          <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
            The memory could not be verified right now. Please retry.
          </p>
        </div>
      </section>

      <section className="mt-10 grid gap-5 md:grid-cols-[minmax(0,1.15fr)_minmax(280px,0.85fr)]">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Status</Badge>
            <CardTitle className="flex items-center gap-3">
              <WifiOff className="size-5" />
              Verification failed
            </CardTitle>
            <CardDescription>The shared memory responded, but its query proof could not be verified.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm text-muted-foreground">
            {memoryId ? <p className="font-mono text-foreground/80">{memoryId}</p> : null}
            <p>Reload the page to try again. This is usually transient.</p>
          </CardContent>
        </Card>

        <Card className="shadow-none">
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Next</Badge>
            <CardTitle>What to do next</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm leading-7 text-muted-foreground">
            <p>Retry this page in a moment.</p>
            <p>If the error persists, verify IC network or gateway health before treating it as an ACL issue.</p>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
