// Where: shadcn-style input primitive for the share hub.
// What: standardizes single-line text input styling and focus behavior.
// Why: auxiliary forms and future controls should share one owned input implementation.

import * as React from "react";
import { cn } from "@/lib/utils";

export function Input({ className, type, ...props }: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      type={type}
      className={cn(
        "flex h-11 w-full rounded-full border border-input bg-background px-4 py-2 text-sm text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-accent/50 focus-visible:ring-2 focus-visible:ring-ring/70 focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
