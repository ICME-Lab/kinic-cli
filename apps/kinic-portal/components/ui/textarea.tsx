// Where: shadcn-style textarea primitive for the share hub.
// What: standardizes multiline input styling and focus behavior.
// Why: public search/chat uses one shared accessible input control.

import * as React from "react";
import { cn } from "@/lib/utils";

export function Textarea({ className, ...props }: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        "flex min-h-32 w-full rounded-[24px] border border-input bg-background px-5 py-4 text-base text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-accent/50 focus-visible:ring-2 focus-visible:ring-ring/70 focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        className,
      )}
      {...props}
    />
  );
}
