export class FormBodyError extends Error {
  constructor(
    readonly status: 400 | 413 | 415,
    message: string
  ) {
    super(message);
    this.name = "FormBodyError";
  }
}

export async function readBoundedUrlEncodedForm(
  request: Request,
  maxBytes: number
): Promise<URLSearchParams> {
  const mediaType = request.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase();
  if (mediaType !== "application/x-www-form-urlencoded") {
    throw new FormBodyError(415, "expected application/x-www-form-urlencoded");
  }

  const rawLength = request.headers.get("content-length");
  if (rawLength !== null) {
    if (!/^\d+$/.test(rawLength)) throw new FormBodyError(400, "invalid Content-Length");
    if (Number(rawLength) > maxBytes) throw new FormBodyError(413, "payload too large");
  }

  if (!request.body) return new URLSearchParams();

  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > maxBytes) {
      try {
        await reader.cancel("payload too large");
      } catch {
        // A source cancellation error must not obscure the enforced size limit.
      }
      throw new FormBodyError(413, "payload too large");
    }
    chunks.push(value);
  }

  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new URLSearchParams(new TextDecoder().decode(bytes));
}
