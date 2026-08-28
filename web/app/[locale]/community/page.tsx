import Link from "next/link";
import { getFacts } from "@/lib/facts";
import { buildPageMetadata } from "@/lib/page-meta";
import { RELEASE_CONTRIBUTORS, RELEASE_HELPERS } from "@/lib/release-credits";

export const revalidate = 300;

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  return buildPageMetadata({
    path: "/community",
    locale,
    title: isZh ? "社区 · Ghosty" : "Community · Ghosty",
    description: isZh
      ? "了解 Ghosty 的国际开源社区，提交 issue、发送 pull request、改进翻译并查看版本贡献者。"
      : "File issues, send pull requests, improve translations, and see who contributed to each Ghosty release.",
  });
}

export default async function CommunityPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  const p = (path: string) => `/${locale}${path}`;
  const facts = await getFacts();
  const sourceIsPublished = facts.latestPublishedRelease?.version === facts.version;

  const contributionPaths = isZh
    ? [
        {
          title: "报告问题",
          description: "报告 bug、兼容性问题或不清楚的行为，并附上系统信息、复现步骤和可以安全分享的日志。",
          cta: "提交 issue →",
          href: "https://github.com/blissito/ghostycode/issues/new/choose",
        },
        {
          title: "改进代码或测试",
          description: "挑一个范围清楚的问题，写最小的补丁，加一个覆盖改动的回归测试。",
          cta: "查看开放 issues →",
          href: "https://github.com/blissito/ghostycode/issues",
        },
        {
          title: "改进文档或翻译",
          description: "改正说错的地方，补一个示例，或者帮忙完成一个语言包。",
          cta: "查看本地化指南 ↗",
          href: "https://github.com/blissito/ghostycode/blob/main/docs/LOCALIZATION.md",
        },
        {
          title: "复现并审查现有工作",
          description: "在你的平台和提供商上验证 issue 或 pull request，然后分享你运行的命令、结果和剩余问题。",
          cta: "查看 pull requests →",
          href: "https://github.com/blissito/ghostycode/pulls",
        },
      ]
    : [
        {
          title: "Report a problem",
          description: "File a bug, compatibility problem, or unclear behavior with system details, reproduction steps, and any logs you can share safely.",
          cta: "File an issue →",
          href: "https://github.com/blissito/ghostycode/issues/new/choose",
        },
        {
          title: "Improve code or tests",
          description: "Pick one problem with clear edges, write the smallest patch that fixes it, and add a regression test that covers it.",
          cta: "Browse open issues →",
          href: "https://github.com/blissito/ghostycode/issues",
        },
        {
          title: "Improve documentation or translations",
          description: "Fix a wrong sentence, add an example, or help finish a language pack.",
          cta: "Open the localization guide ↗",
          href: "https://github.com/blissito/ghostycode/blob/main/docs/LOCALIZATION.md",
        },
        {
          title: "Reproduce and review existing work",
          description: "Try an issue or pull request on your platform and provider. Post the commands you ran, what happened, and what is still wrong.",
          cta: "Browse pull requests →",
          href: "https://github.com/blissito/ghostycode/pulls",
        },
      ];

  const activityLinks = isZh
    ? [
        { title: "仓库动态", description: "最近的 issues 与 pull requests。", href: p("/feed") },
        { title: "社区摘要", description: "经过维护者审核的每周项目记录。", href: p("/digest") },
        { title: "公开路线图", description: "已发布、正在进行、考虑中和明确不做的工作。", href: p("/roadmap") },
      ]
    : [
        { title: "Repository activity", description: "Recent issues and pull requests.", href: p("/feed") },
        { title: "Community digest", description: "The weekly project record, reviewed by a maintainer.", href: p("/digest") },
        { title: "Public roadmap", description: "Shipped, underway, considered, and ruled-out work.", href: p("/roadmap") },
      ];

  return (
    <>
      <section className="hero">
        <div className="portal-current" aria-hidden="true" />
        <div className="portal-container community-welcome-inner">
          <div className="eyebrow">{isZh ? "国际开源社区" : "International open-source community"}</div>
          <h1>{isZh ? "与世界各地的贡献者一起构建 Ghosty。" : "Build Ghosty with contributors around the world."}</h1>
          <p>
            {isZh
              ? "运行时、文档、测试和翻译，来自不同国家、语言和平台的贡献者。第一次参与不必是大功能。一份清楚的 bug 报告、一处文档修正、或一个带测试的小补丁，都算。"
              : "The runtime, docs, tests, and translations come from contributors across countries, languages, and platforms. A first contribution does not have to be a feature. A clear bug report, a documentation fix, or a small tested patch counts."}
          </p>
          <div className="portal-actions">
            <Link href="https://github.com/blissito/ghostycode/issues/new/choose" className="portal-button portal-button-primary">
              {isZh ? "提交 issue" : "File an issue"}
            </Link>
            <Link href="https://github.com/blissito/ghostycode/pulls" className="portal-button portal-button-secondary">
              {isZh ? "查看 pull requests" : "Browse pull requests"}
            </Link>
            <Link href={p("/contribute")} className="portal-button portal-button-secondary">
              {isZh ? "阅读贡献指南" : "Read the contribution guide"}
            </Link>
          </div>
        </div>
      </section>

      <section className="portal-section">
        <div className="portal-container portal-section-grid">
          <div className="portal-section-copy">
            <span>{isZh ? "参与方式" : "Ways to contribute"}</span>
            <h2>{isZh ? "从一件小事开始。" : "Start with one small thing."}</h2>
            <p>
              {isZh
                ? "报 bug、写代码、写测试、改文档、翻译、审查，都有用。选一样适合你时间的。"
                : "Bug reports, code, tests, docs, translations, and review all help. Pick the one that fits the time you have."}
            </p>
          </div>
          <div className="contribute-path-grid">
            {contributionPaths.map((path) => (
              <article key={path.title}>
                <h3>{path.title}</h3>
                <p>{path.description}</p>
                <Link href={path.href}>{path.cta}</Link>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section className="portal-section portal-section-muted">
        <div className="portal-container portal-section-grid">
          <div className="portal-section-copy">
            <span>{isZh ? "公开项目记录" : "Public project record"}</span>
            <h2>{isZh ? "从提案到发布，都是公开的。" : "From proposal to release, in the open."}</h2>
            <p>
              {isZh
                ? "动态页显示最近的仓库活动。社区摘要保留每周存档。路线图区分已发布的和还在讨论的。"
                : "The activity feed shows recent repository work. The community digest keeps the weekly archive of repository activity. The roadmap separates what shipped from what is still being discussed."}
            </p>
          </div>
          <div className="portal-topic-list">
            {activityLinks.map((item) => (
              <Link key={item.href} href={item.href}>
                <strong>{item.title}</strong>
                <span>{item.description}</span>
                <span aria-hidden="true">→</span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      <section className="portal-section community-credit-section">
        <div className="portal-container portal-section-grid">
          <div className="portal-section-copy">
            <span>
              {sourceIsPublished
                ? isZh
                  ? `v${facts.version} 版本致谢`
                  : `v${facts.version} release credit`
                : isZh
                  ? `v${facts.version} 致谢（未发布）`
                  : `v${facts.version} credit (unreleased)`}
            </span>
            <h2>{isZh ? "贡献者署名是版本记录的一部分。" : "Contributor credit is part of the release record."}</h2>
            <p>
              {isZh
                ? `${sourceIsPublished ? "这一版本" : "这一版"}包含社区提交的代码、测试、复现和验证。即使维护者改过补丁再合入，原作者的署名也保留在提交、更新日志和贡献者名单中。`
                : `This ${sourceIsPublished ? "release" : "version"} includes code, tests, reproductions, and verification from the community. If a maintainer reworks a patch before it lands, the original author stays credited in the commit, the changelog, and the contributor record.`}
            </p>
            <div className="community-record-links">
              <Link href="https://github.com/blissito/ghostycode/blob/main/docs/CONTRIBUTORS.md">
                {isZh ? "完整贡献者名单 ↗" : "Full contributor record ↗"}
              </Link>
              <Link href="https://github.com/blissito/ghostycode/blob/main/CHANGELOG.md">CHANGELOG ↗</Link>
            </div>
          </div>
          <div className="community-credit-groups">
            <section>
              <h3>{isZh ? "已合并或吸收的贡献" : "Merged or adapted contributions"}</h3>
              <div className="community-credit-list">
                {RELEASE_CONTRIBUTORS.map((handle) => (
                  <Link key={handle} href={`https://github.com/${handle.slice(1)}`}>{handle}</Link>
                ))}
              </div>
            </section>
            {RELEASE_HELPERS.length > 0 ? (
              <section>
                <h3>{isZh ? "报告、复现与验证" : "Reports, reproductions, and verification"}</h3>
                <div className="community-credit-list">
                  {RELEASE_HELPERS.map((handle) => (
                    <Link key={handle} href={`https://github.com/${handle.slice(1)}`}>{handle}</Link>
                  ))}
                </div>
              </section>
            ) : null}
          </div>
        </div>
      </section>
    </>
  );
}
