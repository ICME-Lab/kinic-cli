// Where: shadcn-style alert primitive for the share hub.
// What: provides a compact bordered message container for error and status output.
// Why: state feedback should share one visual language instead of custom paragraphs.

import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const alertVariants = cva("relative w-full rounded-2xl border px-4 py-3 text-sm shadow-[0_1px_2px_rgba(0,0,0,0.03)]", {
  variants: {
    variant: {
      default: "border-border bg-card text-card-foreground",
      destructive: "border-red-200 bg-red-50/70 text-red-900",
    },
  },
  defaultVariants: {
    variant: "default",
  },
});

export function Alert({
  className,
  variant,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & VariantProps<typeof alertVariants>) {
  return <div className={cn(alertVariants({ variant, className }))} role="alert" {...props} />;
}

export function AlertTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return <h5 className={cn("mb-1 text-sm font-medium leading-none tracking-tight", className)} {...props} />;
}

export function AlertDescription({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("text-sm [&_p]:leading-relaxed", className)} {...props} />;
}
