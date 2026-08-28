/**
 * Flatten a React node tree to searchable / structured-data plain text.
 * Shared by the FAQ search haystack and FAQPage JSON-LD.
 */
export function extractText(node: React.ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string") return node;
  if (typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractText).join(" ");
  if (typeof node === "object" && "props" in node) {
    const props = (node as { props?: { children?: React.ReactNode } }).props;
    return props ? extractText(props.children) : "";
  }
  return "";
}

/** Collapse JSX-extracted whitespace into a single structured-data string. */
export function flattenExtractedText(node: React.ReactNode): string {
  return extractText(node).replace(/\s+/g, " ").trim();
}
