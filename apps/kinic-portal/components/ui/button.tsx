// Where: shadcn-style button primitive for the share hub.
// What: provides accessible button variants built with local Tailwind classes.
// Why: keep UI primitives owned in-repo instead of depending on a runtime component package.

import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-full border text-[15px] font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/80 focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "border-primary bg-primary text-primary-foreground shadow-[0_1px_2px_rgba(0,0,0,0.06)] hover:bg-primary/92",
        secondary: "border-border bg-secondary text-secondary-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] hover:border-input hover:bg-muted",
        outline: "border-input bg-background text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)] hover:border-accent/40 hover:bg-accent/10 hover:text-foreground",
        ghost: "border-transparent bg-transparent text-muted-foreground hover:border-border hover:bg-muted hover:text-foreground",
      },
      size: {
        default: "h-11 px-5",
        sm: "h-9 px-4 text-sm",
        lg: "h-12 px-6",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants>;

export function Button({ className, variant, size, ...props }: ButtonProps) {
  return <button className={cn(buttonVariants({ variant, size, className }))} {...props} />;
}

export { buttonVariants };
