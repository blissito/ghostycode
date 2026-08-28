import * as vscode from "vscode";
import {
  checkRuntime,
  listSnapshots,
  listThreadSummaries,
  openGhostyCodeTerminal,
  readRuntimeConfig,
  runtimeBaseUrl,
  startRuntimeTerminal,
  type RuntimeState,
} from "./runtime";
import { RuntimeStatusView } from "./status";

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("GhostyCode");
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  const statusView = new RuntimeStatusView();
  let autoRefreshTimer: ReturnType<typeof setInterval> | undefined;
  let autoRefreshInFlight = false;

  status.command = "ghosty.checkRuntime";
  context.subscriptions.push(output, status);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(RuntimeStatusView.viewType, statusView),
  );

  const refreshAgentView = async (): Promise<void> => {
    const config = readRuntimeConfig();
    const threads = await listThreadSummaries(config);
    statusView.updateThreads(threads, "Showing recent runtime threads.");
    output.appendLine(`Loaded ${threads.length} runtime thread summaries.`);
  };

  const refreshSnapshots = async (): Promise<void> => {
    const config = readRuntimeConfig();
    const snapshots = await listSnapshots(config);
    statusView.updateSnapshots(snapshots, "Showing recent restore points.");
    output.appendLine(`Loaded ${snapshots.length} runtime restore points.`);
  };

  const refreshAgentViewDetails = async (showWarning: boolean): Promise<void> => {
    try {
      await refreshAgentView();
    } catch (error: unknown) {
      const detail = error instanceof Error ? error.message : String(error);
      statusView.updateThreads([], "Runtime thread summaries unavailable.");
      output.appendLine(`Runtime thread summaries unavailable: ${detail}`);
      if (showWarning) {
        void vscode.window.showWarningMessage(detail);
      }
    }

    try {
      await refreshSnapshots();
    } catch (error: unknown) {
      const detail = error instanceof Error ? error.message : String(error);
      statusView.updateSnapshots([], detail);
      output.appendLine(`Runtime restore points unavailable: ${detail}`);
      if (showWarning) {
        void vscode.window.showWarningMessage(detail);
      }
    }
  };

  const updateStatus = (text: string, tooltip: string): void => {
    status.text = text;
    status.tooltip = tooltip;
    status.show();
  };

  const checkAndRefreshRuntime = async (
    showSpinner: boolean,
    logResult: boolean,
  ): Promise<RuntimeState> => {
    const config = readRuntimeConfig();
    if (showSpinner) {
      updateStatus("$(sync~spin) GhostyCode", "Checking GhostyCode runtime...");
    }

    const state = await checkRuntime(config);
    statusView.update(state);

    switch (state.kind) {
      case "connected":
        updateStatus("$(check) GhostyCode", state.detail);
        await refreshAgentViewDetails(false);
        break;
      case "auth-required":
        updateStatus("$(lock) GhostyCode", state.detail);
        statusView.updateThreads([], "Runtime token is required before threads can load.");
        statusView.updateSnapshots([], "Runtime token is required before restore points can load.");
        break;
      case "offline":
      case "error":
        updateStatus("$(warning) GhostyCode", state.detail);
        statusView.updateThreads([], "Connect to the runtime to load recent threads.");
        statusView.updateSnapshots([], "Connect to the runtime to load restore points.");
        break;
    }

    if (logResult) {
      output.appendLine(`${new Date().toISOString()} ${state.kind}: ${state.detail}`);
    }
    return state;
  };

  const runAutoRefresh = async (): Promise<void> => {
    if (autoRefreshInFlight) {
      return;
    }

    autoRefreshInFlight = true;
    try {
      await checkAndRefreshRuntime(false, false);
    } finally {
      autoRefreshInFlight = false;
    }
  };

  const scheduleAutoRefresh = (): void => {
    if (autoRefreshTimer) {
      clearInterval(autoRefreshTimer);
      autoRefreshTimer = undefined;
    }

    const intervalSeconds = readRuntimeConfig().agentViewRefreshIntervalSeconds;
    if (intervalSeconds === 0) {
      output.appendLine("Agent View auto-refresh is disabled.");
      return;
    }

    autoRefreshTimer = setInterval(() => {
      void runAutoRefresh();
    }, intervalSeconds * 1000);
    output.appendLine(`Agent View auto-refresh scheduled every ${intervalSeconds}s.`);
  };

  updateStatus("$(terminal) GhostyCode", "Check GhostyCode runtime");
  scheduleAutoRefresh();
  context.subscriptions.push(
    new vscode.Disposable(() => {
      if (autoRefreshTimer) {
        clearInterval(autoRefreshTimer);
      }
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("ghosty.agentViewRefreshIntervalSeconds")) {
        scheduleAutoRefresh();
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("ghosty.openTerminal", () => {
      const config = readRuntimeConfig();
      openGhostyCodeTerminal(config);
      output.appendLine(`Opened GhostyCode terminal using ${config.commandPath}.`);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("ghosty.startRuntime", () => {
      const config = readRuntimeConfig();
      startRuntimeTerminal(config);
      const baseUrl = runtimeBaseUrl(config);
      updateStatus("$(sync~spin) GhostyCode", `Runtime terminal started for ${baseUrl}`);
      output.appendLine(`Started GhostyCode runtime terminal at ${baseUrl}.`);
      void vscode.window.showInformationMessage(`GhostyCode runtime starting at ${baseUrl}`);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("ghosty.checkRuntime", async () => {
      return await checkAndRefreshRuntime(true, true);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("ghosty.refreshAgentView", async () => {
      await refreshAgentViewDetails(true);
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("ghosty.refreshSnapshots", async () => {
      try {
        await refreshSnapshots();
      } catch (error: unknown) {
        const detail = error instanceof Error ? error.message : String(error);
        statusView.updateSnapshots([], detail);
        output.appendLine(`Runtime restore points unavailable: ${detail}`);
        void vscode.window.showWarningMessage(detail);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("ghosty.openRuntimeDocs", () => {
      void vscode.env.openExternal(
        vscode.Uri.parse(
          "https://github.com/blissito/ghostycode/blob/main/docs/RUNTIME_API.md",
        ),
      );
    }),
  );

  void vscode.commands.executeCommand("ghosty.checkRuntime");
}

export function deactivate(): void {
  // No background process is owned by the extension; runtime starts in a user-visible terminal.
}
