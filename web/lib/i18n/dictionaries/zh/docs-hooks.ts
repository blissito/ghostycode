import type { DocsHooksDict } from "../types";

/** Simplified Chinese dictionary for `app/[locale]/docs/hooks/page.tsx`. */
export const docsHooks: DocsHooksDict = {
  metaTitle: "钩子 · Ghosty 文档",
  metaDescription:
    "已发布的生命周期钩子：可变 message_submit、tool_call_before 决策、turn_end 与子 Agent 观察事件。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "钩子",
  overviewLead:
    "钩子让你把自己的命令挂进 Ghosty 的生命周期：在消息提交前注入上下文、在工具调用前执行策略、在回合结束或子 Agent 启停时做审计。本页描述当前已发布的行为；docs/rfcs/1364-hooks-lifecycle.md 是这组能力的设计 RFC，完整配置 schema 见 docs/CONFIGURATION.md。",
  configIntro:
    "钩子配置在 config.toml 的 {hooksTable} 条目下；TUI 里运行 {hooksCommand} 可以按事件分组查看每个钩子的名称、命令预览、超时和条件，以及 {enabledKey} 的全局开关状态。",
  events: [
    [
      "message_submit（可变）",
      "在用户消息进入历史或发给模型之前运行。钩子从 stdin 收到 JSON；exit 0 且 stdout 打印含非空 text 字段的 JSON 时替换提交文本；exit 2 在回合开始前阻止提交。多个钩子按配置顺序串行执行，每个钩子收到上一个钩子的输出文本。标记 background = true 的钩子只能观察，不能改写或阻止。",
    ],
    [
      "tool_call_before（决策）",
      "在每次工具调用执行前运行。除 exit 2 硬拒绝（始终生效）外，前台钩子可在 exit 0 时用 stdout JSON 给出决策：allow / deny / ask，并可附带 updatedInput 改写工具输入、additionalContext 追加进给模型的工具结果。多个钩子命中时优先级为 deny > ask > allow；tool_name 条件支持 * 通配（如 mcp__* 匹配所有 MCP 工具）。Full Access 不打开工具审批提示，因此 ask 不会降低该姿态。",
    ],
    [
      "turn_end（观察）",
      "在每个模型回合结束后触发，此时用量、成本、通知、收据和队列恢复状态都已更新。stdin 收到包含 status、duration_ms、usage、totals、queued_message_count 等字段的 JSON。stdout 被忽略，失败只记警告——不能阻止输入、改写 transcript 或改变下一个排队消息。",
    ],
    [
      "subagent_spawn / subagent_complete（观察）",
      "观察子 Agent 的启动与完成，stdin 收到有界的 JSON 元数据（agent_id、状态、截断后的 prompt/result 预览）。失败只记警告，不阻塞调度、不改 prompt 或结果；需要完整细节时使用 agent 返回的 transcript 句柄。",
    ],
  ],
  projectTitle: "项目级钩子",
  projectLead:
    '仓库可以在 <workspace>/.ghosty/hooks.toml 中携带策略。因为项目钩子是可执行的 shell 配置，Ghosty 只有在工作区通过信任提示或用户配置中的 trust_level = "trusted" 被信任后才加载它们——会话内的 /trust on 和旧版 .deepseek/trusted 标记都不会单独启用项目钩子。受信任后，项目钩子追加在 config.toml 的全局钩子之后运行，因此对 updatedInput 而言最后生效。格式错误但已受信任的项目文件会记警告并回退到只用全局钩子。',
  sourceNote:
    "来源文档：docs/rfcs/1364-hooks-lifecycle.md（设计 RFC）, docs/CONFIGURATION.md（配置 schema）· 更新时请同步修改 docs-map.ts。",
};
