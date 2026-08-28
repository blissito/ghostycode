import { Fragment } from "react";
import { getDocsRuntimeApi, splitTokens } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

/**
 * The command each entry row is headed by, and the flags and environment
 * variables the security paragraph typesets as inline `<code>`. `docs/VOICE.md`
 * keeps commands, key names and paths code-owned, so the dictionaries carry
 * keys and `{token}`s rather than the literals.
 */
const ENTRY_COMMANDS: Record<string, string> = {
  http: "ghosty app-server --http",
  mobile: "ghosty app-server --mobile",
  stdio: "ghosty app-server --stdio",
  web: "ghosty web [--port 7878]",
  doctor: "ghosty doctor --json",
  acp: "ghosty serve --acp",
  exec: "ghosty exec [args]",
};

const CODE_SPANS: Record<string, string> = {
  authToken: "--auth-token",
  runtimeTokenEnv: "GHOSTY_RUNTIME_TOKEN",
  legacyTokenEnv: "DEEPSEEK_RUNTIME_TOKEN",
  insecureFlag: "--insecure-no-auth",
  mobileFlag: "app-server --mobile",
};

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsRuntimeApi(locale);
  return buildPageMetadata({
    path: "/docs/runtime-api",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function RuntimeApiPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsRuntimeApi(locale);

  return (
    <section className="space-y-10">
      <section id="overview" className="scroll-mt-32">
        <h1 className="font-display text-3xl mb-1">{t.overviewTitle}</h1>
        <p className={`${t.bodyClassName} mt-3`}>{t.overviewLead}</p>
        <div className="hairline-t mt-6">
          {t.entries.map(([key, detail]) => (
            <section key={key} className="py-4 hairline-b">
              <h3 className="font-mono text-sm font-semibold">{ENTRY_COMMANDS[key] ?? key}</h3>
              <p className={`${t.bodyClassName} mt-1 text-sm`}>{detail}</p>
            </section>
          ))}
        </div>
      </section>

      <section id="stdio" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">{t.stdioTitle}</h2>
        <p className={`${t.bodyClassName} mt-3`}>{t.stdioLead}</p>
        <pre className="code-block mt-4">{`printf '%s\n' \\
  '{"jsonrpc":"2.0","id":1,"method":"healthz"}' \\
  '{"jsonrpc":"2.0","id":2,"method":"capabilities"}' \\
  '{"jsonrpc":"2.0","id":3,"method":"shutdown"}' \\
  | ghosty app-server --stdio`}</pre>
        <p className={`${t.bodyClassName} mt-3`}>{t.interruptNote}</p>
      </section>

      <section id="security" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">{t.securityTitle}</h2>
        <p className={`${t.bodyClassName} mt-3`}>
          {splitTokens(t.securityLead).map((part, i) =>
            "token" in part ? (
              <code key={`${i}-${part.token}`} className="inline">
                {CODE_SPANS[part.token] ?? `{${part.token}}`}
              </code>
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
