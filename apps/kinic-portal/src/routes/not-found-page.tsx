// Where: browser and Worker fallback route for unsupported portal paths.
// What: returns one minimal not-found screen outside the memory access states.
// Why: only `/` and `/m/:memoryId` are valid public routes for this shell.

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function NotFoundPage() {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="max-w-4xl space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <Badge variant="secondary">Not Found</Badge>
          </div>
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Portal Route</p>
          <h1 className="text-[clamp(2.6rem,6vw,4.2rem)] font-semibold leading-[1.05] tracking-[-0.04em] text-foreground">
            Page not found.
          </h1>
          <p className="max-w-3xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
            This public portal only exposes the landing page and shared memory routes.
          </p>
        </div>
      </section>

      <section className="mt-10">
        <Card>
          <CardHeader className="gap-3">
            <Badge variant="secondary" className="w-fit">Available Routes</Badge>
            <CardTitle>`/` and `/m/:memoryId`</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm leading-7 text-muted-foreground">
            <p>Use a valid public memory URL or return to the landing page.</p>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
