import { DocsSearch } from "@/components/docs-search";
import { getDocsShell } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsShell(locale);
  return buildPageMetadata({
    path: "/docs",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function DocsHubPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsShell(locale);
  return (
    <>
      {/* The hub's own heading. The hero line above is shell chrome shared by
          every docs URL, so it is no longer an <h1>; this names the page. */}
      <h1 className="sr-only">{t.portalMark}</h1>
      <DocsSearch locale={locale} />
    </>
  );
}
