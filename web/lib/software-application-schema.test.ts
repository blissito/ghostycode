import { describe, expect, it } from "vitest";
import { ORGANIZATION_ID } from "./site-schema";
import { buildSoftwareApplicationJsonLd } from "./software-application-schema";

describe("SoftwareApplication structured data", () => {
  it("uses the published version advertised by the release-backed install URL", () => {
    const schema = buildSoftwareApplicationJsonLd({ version: "0.9.3" });

    expect(schema.downloadUrl).toBe("https://ghosty.net/en/install");
    expect(schema.softwareVersion).toBe("0.9.3");
    expect(schema.author).toEqual({ "@id": ORGANIZATION_ID });
  });

  it("omits softwareVersion when no published release receipt is available", () => {
    const schema = buildSoftwareApplicationJsonLd(null);

    expect(schema).not.toHaveProperty("softwareVersion");
    expect(schema.author).toEqual({ "@id": ORGANIZATION_ID });
  });
});
