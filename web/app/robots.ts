import type { MetadataRoute } from "next";
import { SITE_URL } from "@/lib/page-meta";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: ["/", "/llms.txt"],
        disallow: ["/api/", "/*/admin"],
      },
    ],
    sitemap: `${SITE_URL}/sitemap.xml`,
  };
}
