import { NextResponse } from "next/server";
import { fetchFeed } from "@/lib/github";

export const revalidate = 600;
export const dynamic = "force-static";

export async function GET() {
  // This route is public. Never spend the server-held GitHub token on behalf
  // of an unauthenticated caller; scheduled/private tasks use it separately.
  const items = await fetchFeed(undefined, 50);
  return NextResponse.json({ items, fetchedAt: new Date().toISOString() });
}
