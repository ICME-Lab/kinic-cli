// Where: composed UI component for shared memory metadata stats.
// What: wraps one labeled value in a card-shaped stat tile.
// Why: memory and landing pages should reuse one stat presentation instead of duplicating markup.

import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export function MemoryStat({
  label,
  value,
  className,
}: {
  label: string;
  value: string;
  className?: string;
}) {
  return (
    <Card className={cn("grid gap-2 rounded-2xl px-4 py-4 shadow-none", className)}>
      <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">{label}</span>
      <strong className="break-all font-mono text-sm font-medium text-foreground">{value}</strong>
    </Card>
  );
}
