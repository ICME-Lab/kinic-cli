// Where: public memory route shell in the browser router tree.
// What: resolves the memory id from the URL and passes runtime origins into the existing client fetch UI.
// Why: the body stays client-driven even though the Worker now server-renders the initial HTML shell and metadata.

import { useParams } from "react-router";
import { PublicMemoryPage } from "@/components/public-memory-page";
import type { PortalRuntimeConfig } from "../runtime-config";
import { NotFoundPage } from "./not-found-page";

export function MemoryPage({ config }: { config: PortalRuntimeConfig }) {
  const params = useParams();
  const memoryId = params.memoryId?.trim();

  if (!memoryId) {
    return <NotFoundPage />;
  }

  return (
    <PublicMemoryPage
      memoryId={memoryId}
      mcpEndpoint={config.mcpEndpoint}
      publicApiOrigin={config.publicApiOrigin}
    />
  );
}
