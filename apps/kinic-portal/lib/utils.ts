// Where: shared UI utility for shadcn-style components.
// What: merges conditional class names and resolves Tailwind class conflicts.
// Why: shadcn components depend on a single `cn()` helper for local customization.

import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
