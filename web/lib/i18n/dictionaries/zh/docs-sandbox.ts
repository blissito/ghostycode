import type { DocsSandboxDict } from "../types";

/** 中文对照见 `en/docs-sandbox.ts`,文案自页面的 `isZh` 三元逐字迁入。 */
export const docsSandbox: DocsSandboxDict = {
  metaTitle: "沙箱与审批 · Ghosty 文档",
  metaDescription: "macOS Seatbelt、Linux 可选 bubblewrap、平台缺口和审批策略的真实边界。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "沙箱与审批",
  overviewLead:
    "Ghosty 可以启动由模型提出的 shell 命令。审批策略、感知工作区的文件工具和操作系统命令包装器是三个独立的控制：一次审批不是沙箱，选择 workspace-write 也不代表当前平台有可用的 OS 包装器。本页只描述已经接入命令执行路径的行为。",
  platforms: [
    [
      "macOS · Seatbelt",
      "Ghosty 探测 /usr/bin/sandbox-exec；探测成功且策略要求沙箱时，子命令会被包上运行时生成的 Seatbelt profile：广泛的文件系统读取、按策略限制的写入、仅在策略允许时放行网络。探测失败则如实报告无 OS 沙箱。",
    ],
    [
      "Linux · 可选 bubblewrap",
      "Linux 命令沙箱是显式启用的：设置 prefer_bwrap = true，且 /usr/bin/bwrap 是可执行文件时才选用。子命令得到只读根视图，writable 挂载来自解析后的策略；默认隔离网络命名空间，仅在策略开启 network_access 时加 --share-net。未启用或未安装 bwrap 时报告 none。",
    ],
    [
      "Windows · 无 OS 沙箱",
      "Windows 命令路径目前报告无 OS 沙箱。主机权限和审批策略仍然有效，但它们不是 Ghosty 的 OS 命令沙箱。",
    ],
    [
      "外部 OpenSandbox 执行",
      '配置 sandbox_backend = "opensandbox" 后，shell 执行会发往配置的 OpenSandbox 兼容 HTTP 端点，而不是启动本地子进程。隔离保证属于所配置的服务及其运营者。',
    ],
  ],
  policiesTitle: "策略与回退",
  policiesLead:
    "本地 {sandboxMode} 取值为 {readOnly}、{workspaceWrite}、{dangerFullAccess} 或 {externalSandbox}。前两者只在选中且可用的 Seatbelt 或 bubblewrap 包装器下被强制执行；{dangerFullAccess} 有意绕过本地 OS 包装器；{externalSandbox} 声明执行已被外部隔离。没有选中包装器时，shell 命令在没有 Ghosty OS 隔离的情况下运行——审批规则和感知工作区的原生文件工具仍是独立的控制。",
  diagnosticsTitle: "诊断与限制",
  diagnosticsLead:
    "ghosty setup --status、ghosty doctor、ghosty doctor --json 和 diagnostics 工具会报告应用 bubblewrap 偏好后本地可用的包装器。拒绝归因是保守的：子命令的通用 Permission denied 本身并不能证明是 Ghosty 的沙箱拦截了它，未沙箱化的命令失败永远不会被标记为沙箱拒绝。",
  diagnosticsLimits:
    "限制同样如实说明：可用性在启动前检查，选中的包装器仍可能因主机策略、容器限制或竞态而失败；bubblewrap 会忽略缺失或不是目录的可写根；没有任何沙箱能防御内核漏洞或所有资源耗尽与侧信道攻击。",
  sourceNote: "来源文档：docs/SANDBOX.md · 更新时请同步修改 docs-map.ts。",
};
