import Link from "next/link";
import { getFacts } from "@/lib/facts";
import { buildPageMetadata } from "@/lib/page-meta";

export const revalidate = 300;

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  return buildPageMetadata({
    path: "/models",
    locale,
    title: isZh ? "模型与提供商 · Ghosty" : "Models & providers · Ghosty",
    description: isZh
      ? "Ghosty 支持的托管与本地提供商：如何配置，以及完整列表。"
      : "Every hosted and local provider Ghosty supports, and how to configure one.",
  });
}

export default async function ModelsPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  const p = (path: string) => (isZh ? `/zh${path}` : `/en${path}`);
  const facts = await getFacts();
  const providerDocs = "https://github.com/blissito/ghostycode/blob/main/docs/PROVIDERS.md";

  const setupPatterns = isZh
    ? [
        {
          title: "DeepSeek",
          detail: `新配置默认使用 ${facts.defaultModel ?? "deepseek-v4-pro"}。用 --provider、/provider 或 GHOSTY_PROVIDER 换成别的提供商。`,
          reference: "DEEPSEEK_API_KEY",
        },
        {
          title: "本地运行时",
          detail: "vLLM、SGLang 和 Ollama 直连 localhost。设置端点和模型即可；本地部署通常不需要 API 密钥。",
          reference: "vllm · sglang · ollama",
        },
        {
          title: "OpenRouter",
          detail: "OpenRouter 用一个托管端点访问多个模型。提供商和模型仍由你来选；模型名不会替你切换提供商。",
          reference: "OPENROUTER_API_KEY",
        },
      ]
    : [
        {
          title: "DeepSeek",
          detail: `New installs default to ${facts.defaultModel ?? "deepseek-v4-pro"}. Switch with --provider, /provider, or GHOSTY_PROVIDER.`,
          reference: "DEEPSEEK_API_KEY",
        },
        {
          title: "Local runtimes",
          detail: "vLLM, SGLang, and Ollama connect to localhost. Set an endpoint and a model; local deployments usually need no API key.",
          reference: "vllm · sglang · ollama",
        },
        {
          title: "OpenRouter",
          detail: "OpenRouter is one hosted endpoint for many models. You still pick the provider and the model; a model name never switches the provider for you.",
          reference: "OPENROUTER_API_KEY",
        },
      ];

  return (
    <div className="models-page">
      <section className="hero">
        <div className="portal-current" aria-hidden="true" />
        <div className="portal-container community-welcome-inner">
          <div className="eyebrow">{isZh ? "模型与提供商" : "Models and providers"}</div>
          <h1>{isZh ? "选择模型和提供商。" : "Choose a model and provider."}</h1>
          <p>
            {isZh
              ? `Ghosty 内置 ${facts.providers.length} 个提供商。你来选提供商、模型和端点；每一个都跑同一套本地运行时、工具和权限。托管提供商用你的凭据；本地 vLLM、SGLang 和 Ollama 通常不需要密钥。`
              : `Ghosty ships with ${facts.providers.length} providers. You set the provider, model, and endpoint. Every one runs through the same local runtime, tools, and permissions. Hosted providers use your credentials; local vLLM, SGLang, and Ollama usually need no key.`}
          </p>
          <div className="portal-actions">
            <Link href={providerDocs} className="portal-button portal-button-primary">
              {isZh ? "阅读提供商文档" : "Read the provider docs"}
            </Link>
            <Link href={p("/install")} className="portal-button portal-button-secondary">
              {isZh ? "安装 Ghosty" : "Install Ghosty"}
            </Link>
          </div>
        </div>
      </section>

      <section className="portal-section">
        <div className="portal-container portal-section-grid">
          <div className="portal-section-copy">
            <span>{isZh ? "配置" : "Configuration"}</span>
            <h2>{isZh ? "常用提供商" : "Common providers"}</h2>
            <p>
              {isZh
                ? "托管提供商的密钥用 ghosty auth set 保存，或放进配置文件或环境变量。提供商和模型分开选；模型名不会改变提供商。"
                : "Save a hosted provider's key with ghosty auth set, or set it in config or an environment variable. Provider and model are chosen separately; a model name never changes the provider."}
            </p>
          </div>
          <div className="portal-topic-list">
            {setupPatterns.map((pattern) => (
              <Link key={pattern.title} href={p("/docs/configuration")}>
                <strong>{pattern.title}</strong>
                <span>{pattern.detail}</span>
                <span className="font-mono break-all">{pattern.reference}</span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      <section className="portal-section settings-preview" aria-labelledby="settings-preview-title">
        <div className="portal-container">
          <div className="settings-preview-heading">
            <div>
              <span>{isZh ? "设置界面" : "Settings surface"}</span>
              <h2 id="settings-preview-title">
                {isZh ? "只读设置预览" : "Read-only settings preview"}
              </h2>
            </div>
            <p>
              {isZh
                ? "这是 Ghosty 本地设置的只读预览，不会更改你的本地配置。"
                : "A read-only view of Ghosty's local settings. It does not change your local configuration."}
            </p>
          </div>

          <div className="settings-shell">
            <aside className="settings-rail" aria-label={isZh ? "设置区域" : "Settings areas"}>
              <div className="settings-rail-title">{isZh ? "设置" : "Settings"}</div>
              <ul>
                <li className="settings-rail-item settings-rail-item-active">
                  <span>{isZh ? "模型与提供商" : "Models & providers"}</span>
                  <span>{isZh ? "当前" : "Current"}</span>
                </li>
                <li className="settings-rail-item">{isZh ? "运行时" : "Runtime"}</li>
                <li className="settings-rail-item">{isZh ? "模式" : "Modes"}</li>
                <li className="settings-rail-item">{isZh ? "权限" : "Permissions"}</li>
                <li className="settings-rail-item">{isZh ? "工具与 MCP" : "Tools & MCP"}</li>
              </ul>
            </aside>

            <div className="settings-pane">
              <div className="settings-pane-heading">
                <div>
                  <span>{isZh ? "本地设置" : "Local settings"}</span>
                  <h3>{isZh ? "模型与提供商" : "Models & providers"}</h3>
                </div>
                <span className="settings-readonly-badge">{isZh ? "只读" : "Read only"}</span>
              </div>

              <dl className="settings-default-model">
                <div>
                  <dt>{isZh ? "默认模型" : "Default model"}</dt>
                  <dd>
                    <code className="settings-provider-code">{facts.defaultModel ?? "—"}</code>
                  </dd>
                </div>
                <div>
                  <dt>{isZh ? "提供商" : "Providers"}</dt>
                  <dd>{facts.providers.length}</dd>
                </div>
              </dl>

              <div className="settings-provider-heading">
                <span>{isZh ? "提供商" : "Provider"}</span>
                <span>{isZh ? "环境变量" : "Environment variable"}</span>
              </div>
              <ul className="settings-provider-list">
                {facts.providers.map((provider) => (
                  <li key={provider.id}>
                    <div>
                      <strong>{provider.label}</strong>
                      <code className="settings-provider-code">{provider.id}</code>
                    </div>
                    <div className="settings-provider-auth">
                      <span className="settings-registry-marker" aria-hidden="true" />
                      <code className="settings-provider-code">{provider.env}</code>
                    </div>
                  </li>
                ))}
              </ul>

              <div className="settings-docs-action">
                <p>
                  {isZh
                    ? "要在你的机器上设置提供商、模型、端点或凭据，请看配置文档。"
                    : "To set a provider, model, endpoint, or credentials on your machine, see the configuration docs."}
                </p>
                <Link href={p("/docs/configuration")}>
                  {isZh ? "打开配置文档 ↗" : "Open configuration docs ↗"}
                </Link>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="portal-section portal-section-muted">
        <div className="portal-container">
          <div className="portal-docs-heading">
            <div>
              <span>{isZh ? "完整列表" : "Full list"}</span>
              <h2>{isZh ? "内置提供商" : "Built-in providers"}</h2>
            </div>
            <Link href={providerDocs}>{isZh ? "打开源文档 ↗" : "Open the source document ↗"}</Link>
          </div>
          <p className={`mb-6 max-w-3xl text-sm text-ink-soft ${isZh ? "leading-[1.9] tracking-wide" : "leading-relaxed"}`}>
            {isZh
              ? "此列表由仓库中的提供商注册表生成，随发布更新。这里列出提供商 ID 和对应的环境变量；协议、默认端点、模型解析和认证优先级见 docs/PROVIDERS.md。"
              : "Generated from the provider registry in the repository and updated with each release. It lists each provider ID and its environment variable. docs/PROVIDERS.md covers wire protocols, default endpoints, model resolution, and authentication precedence."}
          </p>
          <ul className="grid gap-3 sm:grid-cols-2">
            {facts.providers.map((provider) => (
              <li key={provider.id} className="flex items-start gap-3 border hairline rounded-lg bg-paper px-4 py-3 min-w-0">
                <div className="min-w-0">
                  <div className="text-sm text-ink font-medium">{provider.label}</div>
                  <code className="font-mono text-[0.66rem] text-indigo break-all">{provider.id}</code>
                  <div className="mt-1 font-mono text-[0.62rem] text-ink-mute break-all leading-relaxed">
                    <code className="inline">{provider.env}</code>
                  </div>
                </div>
              </li>
            ))}
          </ul>
          <p className={`mt-6 max-w-3xl text-sm text-ink-soft ${isZh ? "leading-[1.9] tracking-wide" : "leading-relaxed"}`}>
            {isZh ? (
              <>
                缺你要的提供商？{" "}
                <Link href="https://github.com/blissito/ghostycode/issues/new/choose" className="body-link">提交 issue</Link>
                ，写明端点、认证方式和模型能力。带注册表、文档和测试的 pull request 也欢迎。
              </>
            ) : (
              <>
                Missing a provider?{" "}
                <Link href="https://github.com/blissito/ghostycode/issues/new/choose" className="body-link">File an issue</Link>
                {" "}with its endpoint, authentication method, and model capabilities. Pull requests that update the registry, docs, and tests are welcome.
              </>
            )}
          </p>
        </div>
      </section>

      <section className="portal-section">
        <div className="portal-container">
          <div className="portal-docs-heading">
            <div>
              <span>{isZh ? "默认值" : "Defaults"}</span>
              <h2>{isZh ? "默认模型与 crate" : "Default model and crates"}</h2>
            </div>
          </div>
          <div className="grid gap-8 sm:grid-cols-2">
            <div>
              <div className="eyebrow mb-1">{isZh ? "默认模型" : "Default model"}</div>
              <code className="inline font-mono text-sm break-all">{facts.defaultModel ?? "—"}</code>
            </div>
            <div>
              <div className="eyebrow mb-1">{isZh ? "Crates" : "Crates"}</div>
              <ul className="flex flex-wrap gap-1.5">
                {facts.crates.map((crate) => (
                  <li key={crate}>
                    <code className="inline font-mono text-[0.68rem] break-all">{crate}</code>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
