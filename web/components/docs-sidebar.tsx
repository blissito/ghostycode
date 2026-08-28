"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { docsTopicIsCurrent } from "@/lib/docs-navigation";
import {
  DOC_CATEGORY_LABELS,
  docTopicHref,
  docTopicIsExternal,
  getTopicsByCategory,
} from "@/lib/docs-map";

export function DocsSidebar({ locale }: { locale: string }) {
  const isZh = locale === "zh";
  const pathname = usePathname();
  const byCategory = getTopicsByCategory();

  return (
    <aside className="docs-sidebar min-w-0">
      <div className="lg:sticky lg:top-24">
        <div className="docs-sidebar-heading">
          <Link href={`/${locale}/docs`}>
            <span>{isZh ? "文档目录" : "Documentation"}</span>
            <span aria-hidden="true">→</span>
          </Link>
        </div>
        <nav aria-label={isZh ? "文档目录" : "Documentation index"}>
          {[...byCategory.entries()].map(([category, topics]) => (
            <div key={category} className="docs-sidebar-group">
              <div className="docs-sidebar-category">
                {isZh ? DOC_CATEGORY_LABELS[category].zh : DOC_CATEGORY_LABELS[category].en}
              </div>
              <ul>
                {topics.map((topic) => {
                  const isCurrent = docsTopicIsCurrent(topic, locale, pathname);
                  const isExternal = docTopicIsExternal(topic);
                  return (
                    <li key={topic.id}>
                      <Link
                        href={docTopicHref(topic, locale)}
                        target={isExternal ? "_blank" : undefined}
                        rel={isExternal ? "noreferrer" : undefined}
                        aria-current={isCurrent ? "page" : undefined}
                        className={
                          isCurrent
                            ? "docs-sidebar-link docs-sidebar-link-current"
                            : "docs-sidebar-link"
                        }
                      >
                        <span>{isZh ? topic.label.zh : topic.label.en}</span>
                        {isExternal && <span aria-hidden="true">↗</span>}
                      </Link>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </nav>
      </div>
    </aside>
  );
}
