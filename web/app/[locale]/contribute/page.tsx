import Link from "next/link";
import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  return buildPageMetadata({
    path: "/contribute",
    locale,
    title: isZh ? "参与贡献 · Ghosty" : "Contribute · Ghosty",
    description: isZh
      ? "提交 issue、改进翻译和文档、发送 pull request，参与 Ghosty 国际开源社区。"
      : "File issues, improve translations and documentation, and send pull requests to the international Ghosty community.",
  });
}

const stepsEn = [
  {
    n: "01",
    title: "Choose one clear problem",
    body: "Browse open issues, especially good first issue and help wanted. If the behavior is not tracked yet, open an issue with a reproduction before starting a large change.",
    cta: { label: "Browse open issues", href: "https://github.com/blissito/ghostycode/issues" },
  },
  {
    n: "02",
    title: "Fork and create a branch",
    body: "Clone your fork and use a short branch name such as fix/provider-timeout or docs/fleet-example. Keep unrelated changes in separate pull requests.",
    cta: { label: "Open the repository", href: "https://github.com/blissito/ghostycode" },
  },
  {
    n: "03",
    title: "Test the behavior you changed",
    body: "Run the smallest relevant test first, then formatting and the broader checks required for the part of the repository you touched.",
    cta: { label: "Read the contributor guide", href: "https://github.com/blissito/ghostycode/blob/main/CONTRIBUTING.md" },
  },
  {
    n: "04",
    title: "Explain the result in the PR",
    body: "Describe the problem, the reason for the change, the checks you ran, and any remaining risk. Link the issue when one exists.",
    cta: { label: "View open pull requests", href: "https://github.com/blissito/ghostycode/pulls" },
  },
];

const stepsZh = [
  {
    n: "01",
    title: "选择一个明确的问题",
    body: "先浏览 open issues，尤其是 good first issue 和 help wanted。如果问题尚未记录，请在开始大改动前提交带复现步骤的 issue。",
    cta: { label: "浏览 open issues", href: "https://github.com/blissito/ghostycode/issues" },
  },
  {
    n: "02",
    title: "Fork 并创建分支",
    body: "克隆你的 fork，并使用简短的分支名，例如 fix/provider-timeout 或 docs/fleet-example。无关修改请拆成不同的 pull request。",
    cta: { label: "打开仓库", href: "https://github.com/blissito/ghostycode" },
  },
  {
    n: "03",
    title: "测试你修改的行为",
    body: "先运行最小相关测试，再执行格式检查和你所修改部分需要的更完整检查。",
    cta: { label: "阅读贡献指南", href: "https://github.com/blissito/ghostycode/blob/main/CONTRIBUTING.md" },
  },
  {
    n: "04",
    title: "在 PR 中说明结果",
    body: "说明问题、修改原因、已运行的检查和剩余风险；如果已有 issue，请在 PR 中关联。",
    cta: { label: "查看 open pull requests", href: "https://github.com/blissito/ghostycode/pulls" },
  },
];

export default async function ContributePage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  const steps = isZh ? stepsZh : stepsEn;

  const paths = isZh
    ? [
        {
          title: "报告 bug 或兼容性问题",
          body: "提供系统信息、Ghosty 版本、复现步骤、期望行为和可公开的日志。",
          label: "提交 issue",
          href: "https://github.com/blissito/ghostycode/issues/new/choose",
        },
        {
          title: "改进代码或测试",
          body: "选择一个边界清楚的问题，提交最小补丁，并用回归测试证明修改后的行为。",
          label: "查看待处理 issues",
          href: "https://github.com/blissito/ghostycode/issues",
        },
        {
          title: "改进文档或翻译",
          body: "修正不准确的说明、补充实际示例，或帮助完整语言包保持自然且与英文键一致。",
          label: "阅读本地化指南",
          href: "https://github.com/blissito/ghostycode/blob/main/docs/LOCALIZATION.md",
        },
        {
          title: "复现和审查现有工作",
          body: "验证 issue 或 pull request 在你的平台和提供商上的行为，并分享准确的测试结果。",
          label: "浏览 pull requests",
          href: "https://github.com/blissito/ghostycode/pulls",
        },
      ]
    : [
        {
          title: "Report a bug or compatibility problem",
          body: "Include your system, Ghosty version, reproduction steps, expected behavior, and any logs you can share safely.",
          label: "File an issue",
          href: "https://github.com/blissito/ghostycode/issues/new/choose",
        },
        {
          title: "Improve code or tests",
          body: "Choose a well-bounded problem, make the smallest useful patch, and add a regression test that proves the changed behavior.",
          label: "Browse open issues",
          href: "https://github.com/blissito/ghostycode/issues",
        },
        {
          title: "Improve documentation or translations",
          body: "Correct inaccurate guidance, add a practical example, or help a complete language pack stay natural and aligned with the English keys.",
          label: "Read the localization guide",
          href: "https://github.com/blissito/ghostycode/blob/main/docs/LOCALIZATION.md",
        },
        {
          title: "Reproduce and review existing work",
          body: "Verify an issue or pull request with your platform and provider, then share the exact checks and results.",
          label: "Browse pull requests",
          href: "https://github.com/blissito/ghostycode/pulls",
        },
      ];

  const reviewNotes = isZh
    ? [
        "一个 PR 只解决一个问题，便于测试、审查和保留贡献者署名。",
        "新增行为应有测试；修正文档时，请核对实际命令、配置名和当前版本。",
        "修改认证、凭据、沙箱、发布流程、品牌或全局提示词前，请先提交 issue 并确认范围。",
        "如果维护者需要整理或摘取补丁，原作者仍会在提交、CHANGELOG 和贡献者名单中获得署名。",
      ]
    : [
        "Keep one problem per pull request so the change is easy to test, review, and credit.",
        "Add tests for new behavior. For documentation fixes, verify commands, configuration names, and the current version against the repository.",
        "Open an issue before changing authentication, credentials, sandbox policy, release plumbing, branding, or global prompts.",
        "If a maintainer needs to adapt or harvest a patch, the original contributor remains credited in the commit, changelog, and contributor list.",
      ];

  return (
    <div className="contribute-page">
      <section className="hero">
        <div className="portal-current" aria-hidden="true" />
        <div className="portal-container community-welcome-inner">
          <div className="eyebrow">{isZh ? "参与国际开源社区" : "Contribute to an international open-source project"}</div>
          <h1>{isZh ? "从一个具体的改进开始。" : "Start with one concrete improvement."}</h1>
          <p>
            {isZh
              ? "Ghosty 欢迎来自不同国家、语言、平台和经验水平的贡献者。清楚的 bug 报告、复现结果、文档修正、翻译和小而完整的代码补丁都会直接帮助项目。"
              : "Ghosty welcomes contributors across countries, languages, platforms, and experience levels. Clear bug reports, reproduction results, documentation corrections, translations, and small complete patches all move the project forward."}
          </p>
          <div className="portal-actions">
            <Link href="https://github.com/blissito/ghostycode/issues/new/choose" className="portal-button portal-button-primary">
              {isZh ? "提交 issue" : "File an issue"}
            </Link>
            <Link href="https://github.com/blissito/ghostycode/pulls" className="portal-button portal-button-secondary">
              {isZh ? "查看 pull requests" : "Browse pull requests"}
            </Link>
            <Link href="https://github.com/blissito/ghostycode/blob/main/CONTRIBUTING.md" className="portal-button portal-button-secondary">
              {isZh ? "打开完整贡献指南" : "Open the full contributor guide"}
            </Link>
          </div>
        </div>
      </section>

      <section className="portal-section">
        <div className="portal-container">
          <div className="portal-docs-heading">
            <div>
              <span>{isZh ? "现在就可以参与" : "Ways to help now"}</span>
              <h2>{isZh ? "选择适合你的贡献方式。" : "Choose the contribution that fits."}</h2>
            </div>
          </div>
          <div className="contribute-path-grid">
            {paths.map((path) => (
              <article key={path.title}>
                <h3>{path.title}</h3>
                <p>{path.body}</p>
                <Link href={path.href}>{path.label} →</Link>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section className="portal-section portal-section-muted">
        <div className="portal-container">
          <div className="portal-docs-heading">
            <div>
              <span>{isZh ? "Pull request 流程" : "Pull request workflow"}</span>
              <h2>{isZh ? "从问题到可审查的补丁。" : "From a problem to a reviewable patch."}</h2>
            </div>
          </div>
          <ol className="contribute-steps">
            {steps.map((step) => (
              <li key={step.n}>
                <span>{step.n}</span>
                <div>
                  <h3>{step.title}</h3>
                  <p>{step.body}</p>
                  <Link href={step.cta.href}>{step.cta.label} →</Link>
                </div>
              </li>
            ))}
          </ol>
        </div>
      </section>

      <section className="portal-section">
        <div className="portal-container portal-section-grid">
          <div className="portal-section-copy">
            <span>{isZh ? "审查准备" : "Prepare for review"}</span>
            <h2>{isZh ? "让修改容易验证。" : "Make the change easy to verify."}</h2>
            <p>
              {isZh
                ? "审查者需要看到问题、修改理由、测试证据和剩余风险。范围越清楚，反馈通常越快。"
                : "A reviewer needs the problem, the reason for the change, test evidence, and remaining risk. Clear scope usually leads to clearer feedback."}
            </p>
          </div>
          <ul className="contribute-review-list">
            {reviewNotes.map((note) => <li key={note}>{note}</li>)}
          </ul>
        </div>
      </section>

      <section className="contribute-dev-loop">
        <div className="portal-container portal-section-grid min-w-0">
          <div className="portal-section-copy">
            <span>{isZh ? "本地开发" : "Local development"}</span>
            <h2>{isZh ? "构建并运行相关检查。" : "Build and run the relevant checks."}</h2>
            <p>
              {isZh
                ? "仓库使用 stable Rust。先运行你所修改部分的测试，再运行格式检查、Clippy 和完整工作区测试。"
                : "The repository uses stable Rust. Run the focused test for your change first, followed by formatting, Clippy, and the workspace suite."}
            </p>
          </div>
          <pre className="code-block">
{`git clone https://github.com/YOUR_USERNAME/GhostyCode.git
cd GhostyCode
git checkout -b fix/your-change

cargo build
cargo test -p <owning-crate> --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked`}
          </pre>
        </div>
      </section>
    </div>
  );
}
