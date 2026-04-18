import { initOpenNextCloudflareForDev } from "@opennextjs/cloudflare";

if (process.env.NODE_ENV === "development") {
  initOpenNextCloudflareForDev();
}

/** @type {import("next").NextConfig} */
const nextConfig = {
  experimental: {
    authInterrupts: true,
  },
  typedRoutes: true,
};

export default nextConfig;
