// Where: shared by the default and memory-specific OGP image routes.
// What: renders the Mintlify-like social card shell and compact memory stats.
// Why: keep route image variants visually aligned without duplicating inline layout trees.

import type { CSSProperties, ReactElement } from "react";
import {
  buildMemoryOgpCardModel,
  DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION,
  type MemoryOgpInput,
} from "@kinic/kinic-share";

type OgpImageProps = {
  memory?: MemoryOgpInput;
};

export function renderOgpImage({ memory }: OgpImageProps): ReactElement {
  const card = buildMemoryOgpCardModel(memory ?? {});
  const badges = memory ? ["Public Memory", "Read-only"] : ["Kinic", "Portal"];

  return (
    <div style={frameStyle}>
      <div style={washStyle} />
      <div style={panelStyle}>
        <div style={headerStyle}>
          <div style={brandPillStyle}>Kinic</div>
          <div style={badgeRowStyle}>
            {badges.map((badge) => (
              <div key={badge} style={badgeStyle}>
                {badge}
              </div>
            ))}
          </div>
        </div>

        <div style={heroStyle}>
          <div style={bodyStyle}>
            <div style={eyebrowStyle}>{memory ? "Shared Memory Surface" : "Read-only knowledge surface"}</div>
            <div style={titleStyle}>{card.title}</div>
            <div style={descriptionStyle}>{card.description || DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION}</div>
          </div>
        </div>

        <div style={statsGridStyle}>
          <Stat label="Memory ID" value={card.shortMemoryId} />
        </div>

        <div style={footerStyle}>
          <div style={accentLineStyle} />
          <div style={footerTextStyle}>{memory ? "Read-only knowledge surface" : "Public memory sharing"}</div>
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div style={statCardStyle}>
      <div style={statLabelStyle}>{label}</div>
      <div style={statValueStyle}>{value}</div>
    </div>
  );
}

const frameStyle: CSSProperties = {
  width: "100%",
  height: "100%",
  display: "flex",
  position: "relative",
  background: "#ffffff",
  color: "#0d0d0d",
  fontFamily: "Inter, system-ui, sans-serif",
  overflow: "hidden",
};

const washStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  background:
    "radial-gradient(circle at top center, rgba(24, 226, 153, 0.18), transparent 32%), linear-gradient(180deg, rgba(212, 250, 232, 0.3) 0%, rgba(255, 255, 255, 0.96) 46%, #ffffff 100%)",
};

const panelStyle: CSSProperties = {
  margin: 34,
  padding: 34,
  width: 1132,
  height: 562,
  display: "flex",
  flexDirection: "column",
  justifyContent: "space-between",
  borderRadius: 32,
  border: "1px solid rgba(13, 13, 13, 0.08)",
  background: "rgba(255, 255, 255, 0.88)",
  boxShadow: "0 10px 40px rgba(13, 13, 13, 0.05)",
};

const headerStyle: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
};

const brandPillStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  padding: "10px 14px",
  borderRadius: 999,
  border: "1px solid rgba(13, 13, 13, 0.08)",
  fontSize: 22,
  fontWeight: 600,
};

const badgeRowStyle: CSSProperties = {
  display: "flex",
  gap: 10,
};

const badgeStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  padding: "10px 14px",
  borderRadius: 999,
  background: "rgba(13, 13, 13, 0.04)",
  color: "rgba(13, 13, 13, 0.72)",
  fontSize: 20,
  fontWeight: 500,
};

const bodyStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 20,
  maxWidth: 620,
};

const heroStyle: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  gap: 24,
  alignItems: "stretch",
};

const eyebrowStyle: CSSProperties = {
  fontSize: 18,
  letterSpacing: "0.22em",
  textTransform: "uppercase",
  color: "rgba(13, 13, 13, 0.45)",
};

const titleStyle: CSSProperties = {
  display: "-webkit-box",
  overflow: "hidden",
  fontSize: 72,
  fontWeight: 700,
  lineHeight: 1.02,
  letterSpacing: "-0.04em",
  WebkitBoxOrient: "vertical",
  WebkitLineClamp: 2,
};

const descriptionStyle: CSSProperties = {
  display: "-webkit-box",
  overflow: "hidden",
  maxWidth: 840,
  fontSize: 30,
  lineHeight: 1.45,
  color: "rgba(13, 13, 13, 0.62)",
  WebkitBoxOrient: "vertical",
  WebkitLineClamp: 3,
};

const statsGridStyle: CSSProperties = {
  display: "flex",
  gap: 16,
};

const statCardStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  flex: 1,
  gap: 10,
  padding: "18px 20px",
  borderRadius: 22,
  border: "1px solid rgba(13, 13, 13, 0.06)",
  background: "rgba(255, 255, 255, 0.92)",
};

const statLabelStyle: CSSProperties = {
  fontSize: 18,
  letterSpacing: "0.16em",
  textTransform: "uppercase",
  color: "rgba(13, 13, 13, 0.44)",
};

const statValueStyle: CSSProperties = {
  fontSize: 28,
  fontWeight: 600,
  color: "#0d0d0d",
};

const footerStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 14,
};

const accentLineStyle: CSSProperties = {
  width: 190,
  height: 6,
  borderRadius: 999,
  background: "linear-gradient(90deg, rgba(24, 226, 153, 1) 0%, rgba(24, 226, 153, 0.16) 100%)",
};

const footerTextStyle: CSSProperties = {
  fontSize: 22,
  color: "rgba(13, 13, 13, 0.52)",
};
