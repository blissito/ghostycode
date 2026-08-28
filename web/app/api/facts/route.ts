import { NextResponse } from "next/server";
import { BUILD_FACTS, getFactsWithProvenance, type RepoFacts } from "@/lib/facts";

export const dynamic = "force-dynamic";
export const revalidate = 0;

function summary(facts: RepoFacts) {
  return {
    sourceRevision: facts.sourceRevision,
    sourceCommittedAt: facts.sourceCommittedAt,
    version: facts.version,
    providerCount: facts.providers.length,
    toolCount: facts.toolCount,
  };
}

/** Public, credential-free source/deployment drift receipt. */
export async function GET() {
  const resolution = await getFactsWithProvenance();
  return NextResponse.json(
    {
      schemaVersion: 1,
      deployed: summary(BUILD_FACTS),
      resolved: {
        ...summary(resolution.facts),
        source: resolution.source,
        reason: resolution.reason,
      },
      latestPublishedRelease: resolution.facts.latestPublishedRelease,
    },
    {
      headers: {
        "Cache-Control": "no-store",
      },
    },
  );
}
