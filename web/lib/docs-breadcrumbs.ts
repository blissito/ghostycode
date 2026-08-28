import {
  DOC_CATEGORY_LABELS,
  DOC_TOPICS,
  docTopicHref,
  type DocTopic,
} from "./docs-map";
import { docsTopicIsCurrent } from "./docs-navigation";
import { SITE_URL } from "./page-meta";

export type DocsCrumb = {
  name: string;
  /** Omitted for the current page and for category groupings that have no URL. */
  href?: string;
};

export function resolveDocsTopic(locale: string, pathname: string): DocTopic | undefined {
  return DOC_TOPICS.find((topic) => docsTopicIsCurrent(topic, locale, pathname));
}

function localizedName(pair: { en: string; zh: string }, locale: string): string {
  return locale === "zh" ? pair.zh : pair.en;
}

/** Visible trail: Home → Docs → category → topic. The hub stops at Docs. */
export function resolveDocsBreadcrumbs(locale: string, pathname: string): DocsCrumb[] {
  const home: DocsCrumb = {
    name: locale === "zh" ? "首页" : "Home",
    href: `/${locale}`,
  };
  const docsName = locale === "zh" ? "文档" : "Docs";
  const docsHref = `/${locale}/docs`;
  const topic = resolveDocsTopic(locale, pathname);
  const normalized = pathname.split(/[?#]/)[0].replace(/\/+$/, "");

  if (!topic || normalized === docsHref) {
    return [home, { name: docsName }];
  }

  return [
    home,
    { name: docsName, href: docsHref },
    { name: localizedName(DOC_CATEGORY_LABELS[topic.category], locale) },
    { name: localizedName(topic.label, locale) },
  ];
}

/**
 * BreadcrumbList for the current docs URL.
 * Category groupings have no unique URL, so the machine trail is Home → Docs → topic.
 */
export function buildBreadcrumbListJsonLd(locale: string, pathname: string) {
  const topic = resolveDocsTopic(locale, pathname);
  const elements: { name: string; item: string }[] = [
    {
      name: locale === "zh" ? "首页" : "Home",
      item: `${SITE_URL}/${locale}`,
    },
    {
      name: locale === "zh" ? "文档" : "Docs",
      item: `${SITE_URL}/${locale}/docs`,
    },
  ];
  if (topic) {
    elements.push({
      name: localizedName(topic.label, locale),
      item: `${SITE_URL}${docTopicHref(topic, locale)}`,
    });
  }

  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: elements.map((element, index) => ({
      "@type": "ListItem",
      position: index + 1,
      name: element.name,
      item: element.item,
    })),
  };
}
