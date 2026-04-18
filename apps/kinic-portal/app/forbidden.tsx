// Where: Next.js route-segment fallback for forbidden public pages.
// What: renders the shared anonymous-access denial UI with an actual HTTP 403 status.
// Why: public memory pages should keep their current denial presentation without reporting a 200.

import { MemoryAccessDenied } from "@/components/memory-access-denied";

export default function ForbiddenPage() {
  return <MemoryAccessDenied />;
}
