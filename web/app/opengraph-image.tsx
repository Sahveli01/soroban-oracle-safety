import { ImageResponse } from "next/og";

export const runtime = "edge";
export const alt = "safe-oracle — Trust the oracle. Verify the integrator.";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

/**
 * Dynamic Open Graph image (1200×630 PNG).
 * Renders at request time on the edge. Used by Twitter, Discord,
 * LinkedIn, etc. when the site URL is shared.
 */
export default async function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "#050507",
          padding: 80,
          fontFamily: "monospace",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            marginBottom: 60,
            color: "#8E8E93",
            fontSize: 24,
            letterSpacing: "0.2em",
            textTransform: "uppercase",
          }}
        >
          <div
            style={{
              width: 12,
              height: 12,
              borderRadius: "50%",
              backgroundColor: "#00FF94",
            }}
          />
          safe-oracle
        </div>

        <div
          style={{
            fontSize: 88,
            fontWeight: 500,
            color: "#F5F5F7",
            lineHeight: 1.05,
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            maxWidth: 1000,
          }}
        >
          <span>Trust the oracle.</span>
          <span>Verify the integrator.</span>
        </div>

        <div
          style={{
            fontSize: 28,
            color: "#525258",
            marginTop: 48,
            fontFamily: "monospace",
          }}
        >
          Drop-in oracle protection for Stellar Soroban
        </div>
      </div>
    ),
    { ...size },
  );
}
