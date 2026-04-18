import { defineCloudflareConfig } from "@opennextjs/cloudflare";

export default defineCloudflareConfig({
  enableCacheInterception: false,
  routePreloadingBehavior: "none",
});
