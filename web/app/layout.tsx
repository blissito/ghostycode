import type { Metadata } from "next";
import { SITE_URL } from "@/lib/page-meta";

/**
 * Root metadata boundary. The per-locale `<html>`/`<body>`, fonts, and content
 * metadata live in app/[locale]/layout.tsx; this root only exists to give every
 * route — including the framework-generated `/_not-found` and the root-segment
 * `opengraph-image` — a resolvable `metadataBase` so Next stops falling back to
 * `http://localhost:3000` for social image URLs.
 */
export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return children;
}
