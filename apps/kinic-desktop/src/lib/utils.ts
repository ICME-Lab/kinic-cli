// Where: shared frontend utility helpers.
// What: merges conditional CSS classes.
// Why: keep UI primitives compact without introducing portal imports.

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
