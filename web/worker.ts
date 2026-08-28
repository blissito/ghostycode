import handler from "./.open-next/worker.js";
import {
  runCurate,
  runTriage,
  runPrReview,
  runStale,
  runDupes,
  runDigest,
  type AgentEnv,
} from "./lib/community-agent-tasks";
import { runFactsDrift } from "./lib/facts-drift";
import { runLinkCheck, runSemanticDrift } from "./lib/content-watch";
import { fetchWithStaticInstaller } from "./lib/static-installer";

export default {
  fetch(request, env, ctx) {
    return fetchWithStaticInstaller(
      request,
      env,
      ctx,
      (nextRequest, nextEnv, nextCtx) => handler.fetch(nextRequest, nextEnv, nextCtx),
    );
  },
  async scheduled(event: ScheduledEvent, env: Record<string, unknown>, ctx: ExecutionContext) {
    const expr = event.cron;
    ctx.waitUntil((async () => {
      const agentEnv = env as unknown as AgentEnv;
      if (expr === "0 */6 * * *") {
        await runCurate(agentEnv);
        await runFactsDrift(agentEnv);
      }
      else if (expr === "*/30 * * * *") {
        await runTriage(agentEnv);
        await runPrReview(agentEnv);
      }
      else if (expr === "0 0 * * *") {
        await runStale(agentEnv);
        await runDupes(agentEnv);
        await runLinkCheck(agentEnv);
        await runSemanticDrift(agentEnv);
      }
      else if (expr === "0 9 * * 1") await runDigest(agentEnv);
    })());
  },
} satisfies ExportedHandler;
