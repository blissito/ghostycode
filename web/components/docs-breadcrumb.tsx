"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { buildBreadcrumbListJsonLd, resolveDocsBreadcrumbs } from "@/lib/docs-breadcrumbs";
import { serializeJsonLd } from "@/lib/json-ld";

export function DocsBreadcrumb({ locale }: { locale: string }) {
  const pathname = usePathname() ?? `/${locale}/docs`;
  const crumbs = resolveDocsBreadcrumbs(locale, pathname);
  const jsonLd = buildBreadcrumbListJsonLd(locale, pathname);
  const isZh = locale === "zh";

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: serializeJsonLd(jsonLd) }}
      />
      <nav className="docs-breadcrumb" aria-label={isZh ? "面包屑导航" : "Breadcrumb"}>
        <ol>
          {crumbs.map((crumb, index) => {
            const current = index === crumbs.length - 1;
            return (
              <li key={`${crumb.name}-${index}`}>
                {current || !crumb.href ? (
                  <span aria-current={current ? "page" : undefined}>{crumb.name}</span>
                ) : (
                  <Link href={crumb.href}>{crumb.name}</Link>
                )}
              </li>
            );
          })}
        </ol>
      </nav>
    </>
  );
}
