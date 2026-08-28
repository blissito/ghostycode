import { docTopicHref, type DocTopic } from "./docs-map";

/** Return whether a first-party docs topic owns the current website route. */
export function docsTopicIsCurrent(topic: DocTopic, locale: string, pathname: string): boolean {
  if (!topic.hasPage) return false;

  const normalized = pathname.split(/[?#]/)[0].replace(/\/+$/, "");
  return normalized === docTopicHref(topic, locale);
}
