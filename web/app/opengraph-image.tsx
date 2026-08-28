import { ImageResponse } from "next/og";
import { IDENTITY_PHRASE, OG_ALT } from "@/lib/page-meta";

export const alt = OG_ALT;
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

// The social card mirrors the restrained light-to-depth website treatment.
export default function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          backgroundColor: "#F4F7FB",
          padding: "72px 84px",
          fontFamily: "sans-serif",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 16,
            fontSize: 26,
            letterSpacing: 0,
            textTransform: "uppercase",
            color: "#5B6780",
          }}
        >
          <div style={{ width: 28, height: 14, borderRadius: "50% 45% 45% 50%", backgroundColor: "#F6C453" }} />
          ghosty.net
        </div>
        <div style={{ display: "flex", flexDirection: "column" }}>
          <div
            style={{
              fontSize: 64,
              fontWeight: 700,
              lineHeight: 1.18,
              color: "#14213A",
              maxWidth: 980,
            }}
          >
            {IDENTITY_PHRASE}
          </div>
        </div>
        <div style={{ display: "flex", width: "100%", height: 14, backgroundColor: "#03070D" }}>
          <div style={{ width: "32%", height: "100%", backgroundColor: "#6AAEF2" }} />
        </div>
      </div>
    ),
    { ...size },
  );
}
