import { defineConfig } from "vitest/config";

// Plain Node vitest, matching `web/`'s style — no workers pool, no miniflare.
// The handler is an ordinary `fetch(request, env)` function over standard
// `Request`/`Response`, and every binding it touches is an interface the tests
// stub. That keeps the IP guard in `test/no-ip.test.ts` meaningful: there is no
// runtime shim between the assertion and the source that ships.
export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
  },
});
