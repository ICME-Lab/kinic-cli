// Where: Next.js root layout for the Kinic portal.
// What: defines the shared shell, metadata, and global styles for the public site.
// Why: keep the public experience cohesive while avoiding network-dependent font fetches during build.

import type { Metadata } from "next";
import "./globals.css";
import { buildSiteMetadata } from "@/lib/site-metadata";

export const metadata: Metadata = buildSiteMetadata();

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="font-sans antialiased">{children}</body>
    </html>
  );
}
