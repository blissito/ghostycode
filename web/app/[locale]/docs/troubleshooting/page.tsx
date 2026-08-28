import { getDocsTroubleshooting } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsTroubleshooting(locale);
  return buildPageMetadata({
    path: "/docs/troubleshooting",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function TroubleshootingPage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  const t = getDocsTroubleshooting(locale);

  return (
    <section className="space-y-10">
      <section id="overview" className="scroll-mt-32">
        <h1 className="font-display text-3xl mb-1">{t.overviewTitle}</h1>
        <p className={`${t.bodyClassName} mt-3`}>{t.overviewLead}</p>
        <div className="hairline-t mt-6">
          {t.incidents.map(([name, detail]) => (
            <section key={name} className="py-4 hairline-b">
              <h3 className="font-display text-lg">{name}</h3>
              <p className={`${t.bodyClassName} mt-1 text-sm`}>{detail}</p>
            </section>
          ))}
        </div>
      </section>

      <section id="docker" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">{t.dockerTitle}</h2>
        <p className={`${t.bodyClassName} mt-3`}>{t.dockerLead}</p>
        <pre className="code-block mt-4">{`docker volume create ghosty-home

docker run --rm -it \\
  -e DEEPSEEK_API_KEY="your-api-key-here" \\
  -v ghosty-home:/home/ghosty/.ghosty \\
  -v "$PWD:/workspace" \\
  -w /workspace \\
  ghcr.io/blissito/ghostycode:latest`}</pre>
        <p className={`${t.bodyClassName} mt-3`}>{t.dockerToolboxNote}</p>
      </section>

      <section id="source" className="hairline-t pt-8">
        <p className="text-sm text-ink-mute">{t.sourceNote}</p>
      </section>
    </section>
  );
}
