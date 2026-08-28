import { Fragment } from "react";
import Link from "next/link";
import { getDocsConstitution, splitTokens } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

/**
 * Paths and commands the overview sentence typesets as inline `<code>`.
 * `docs/VOICE.md` keeps these code-owned, so the dictionaries carry a
 * `{token}` for each one instead of the literal.
 */
const CODE_SPANS: Record<string, string> = {
  constitutionCommand: "/constitution",
  homeConfig: "$GHOSTY_HOME/constitution.json",
  repoConfig: ".ghosty/constitution.json",
};

/**
 * The en/zh badge on each principle row. Both halves render in every locale —
 * it is a fixed bilingual glyph rather than copy — so it stays here and the
 * dictionaries key their rows by the same names.
 */
const PRINCIPLE_BADGES: Record<string, [string, string]> = {
  userGlobal: ["User-global", "用户全局"],
  repoLocal: ["Repo-local", "仓库本地"],
  runtime: ["Runtime", "运行时"],
};

const CONFIG_DOCS_HREF =
  "https://github.com/blissito/ghostycode/blob/main/docs/CONFIGURATION.md#constitution-project-instructions-and-repo-authority";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsConstitution(locale);
  return buildPageMetadata({
    path: "/docs/constitution",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function ConstitutionPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsConstitution(locale);

  return (
    <section className="space-y-10">
      <section id="overview" className="scroll-mt-32">
        <h1 className="font-display text-3xl mb-1">
          {t.overviewTitle}{" "}
          <span className="font-cjk text-indigo text-2xl ml-2">{t.overviewTitleAside}</span>
        </h1>
        <p className={`${t.bodyClassName} mt-3`}>
          {splitTokens(t.overviewLead).map((part, i) =>
            "token" in part ? (
              <code key={`${i}-${part.token}`} className="inline">
                {CODE_SPANS[part.token] ?? `{${part.token}}`}
              </code>
            ) : (
              <Fragment key={`${i}-text`}>{part.text}</Fragment>
            ),
          )}
        </p>
        <div className="hairline-t hairline-b mt-6 grid md:grid-cols-3 col-rule">
          {t.principles.map(([key, detail]) => {
            const [name, cn] = PRINCIPLE_BADGES[key] ?? [key, ""];
            return (
              <div key={key} className="p-5">
                <div className="font-display text-lg text-indigo mb-1">
                  {name} <span className="font-cjk text-sm ml-1.5">{cn}</span>
                </div>
                <p className={`text-sm ${t.bodyClassName}`}>{detail}</p>
              </div>
            );
          })}
        </div>
        <p className={`mt-4 text-sm ${t.bodyClassName}`}>
          {splitTokens(t.authorityNote).map((part, i) =>
            "token" in part ? (
              <Link key={`${i}-${part.token}`} href={CONFIG_DOCS_HREF} className="body-link">
                {t.configDocsLabel}
              </Link>
            ) : (
              <Fragment key={`${i}-text`}>{part.text}</Fragment>
            ),
          )}
        </p>
      </section>
      <section id="source" className="hairline-t pt-8">
        <p className="text-sm text-ink-mute">{t.sourceNote}</p>
      </section>
    </section>
  );
}
