/** Serialize JSON-LD so a `</script>` in extracted text cannot break the tag. */
export function serializeJsonLd(value: unknown): string {
  return JSON.stringify(value).replace(/</g, "\\u003c");
}
