// Where: shared by the default and memory-specific OGP image routes.
// What: renders the dark-framed Kinic social card with static image assets and bounded copy.
// Why: bot OGP fetches must stay cheap while matching the supplied frame and logo art.

import type { CSSProperties, ReactElement } from "react";
import {
  buildMemoryOgpCardModel,
  type MemoryOgpInput,
} from "@kinic/kinic-share";
import { OGP_FRAME_SRC, OGP_LOGO_SRC } from "./ogp-assets";
import {
  DEFAULT_SITE_DESCRIPTION,
  DEFAULT_SITE_TITLE,
} from "./site-metadata";

type OgpImageProps = {
  memory?: MemoryOgpInput;
};

export function renderOgpImage({ memory }: OgpImageProps): ReactElement {
  const card = memory
    ? buildMemoryOgpCardModel(memory)
    : {
        title: DEFAULT_SITE_TITLE,
        description: DEFAULT_SITE_DESCRIPTION,
        shortMemoryId: "-",
      };
  return (
    <div style={frameStyle}>
      <img
        alt=""
        src={OGP_FRAME_SRC}
        width={1200}
        height={630}
        style={backgroundImageStyle}
      />
      <div style={panelStyle}>
        <div style={headerStyle}>
          <div style={brandStyle}>
            <KinicMark src={OGP_LOGO_SRC} />
            <div style={brandTextStyle}>KinicMemory</div>
          </div>
        </div>

        <div style={heroStyle}>
          <div style={bodyStyle}>
            <div style={titleStyle}>{card.title}</div>
            <div style={descriptionStyle}>{card.description}</div>
            <div style={statsGridStyle}>
              <Stat label="MEMORY ID" value={card.shortMemoryId} />
            </div>
          </div>
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
  background: "#060c1b",
  color: "#f7fbff",
  fontFamily: "system-ui, sans-serif",
  overflow: "hidden",
};

const backgroundImageStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  width: "100%",
  height: "100%",
};

const panelStyle: CSSProperties = {
  margin: 0,
  padding: "48px 60px 52px",
  width: "100%",
  height: "100%",
  display: "flex",
  flexDirection: "column",
  gap: 36,
  position: "relative",
};

const headerStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
};

const brandStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 18,
};

const brandTextStyle: CSSProperties = {
  fontSize: 40,
  fontWeight: 600,
  color: "#dbe9ff",
  letterSpacing: "-0.03em",
};

const bodyStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 22,
  width: 940,
};

const heroStyle: CSSProperties = {
  display: "flex",
  gap: 18,
  alignItems: "flex-start",
  marginTop: 10,
};

const titleStyle: CSSProperties = {
  display: "-webkit-box",
  overflow: "hidden",
  fontSize: 64,
  fontWeight: 700,
  lineHeight: 1.04,
  letterSpacing: "-0.04em",
  WebkitBoxOrient: "vertical",
  WebkitLineClamp: 2,
  color: "#ffffff",
};

const descriptionStyle: CSSProperties = {
  display: "-webkit-box",
  overflow: "hidden",
  width: 940,
  fontSize: 29,
  lineHeight: 1.34,
  color: "rgba(235, 241, 252, 0.92)",
  WebkitBoxOrient: "vertical",
  WebkitLineClamp: 3,
};

const statsGridStyle: CSSProperties = {
  display: "flex",
  gap: 16,
  width: 520,
  marginTop: 10,
};

const statCardStyle: CSSProperties = {
  display: "flex",
  flexDirection: "row",
  alignItems: "center",
  gap: 14,
  padding: 0,
  background: "transparent",
};

const statLabelStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  lineHeight: 1,
  fontSize: 20,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "#9db4d6",
};

const statValueStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  lineHeight: 1,
  fontSize: 26,
  fontWeight: 500,
  color: "#eff4ff",
};

function KinicMark({ src }: { src: string }) {
  return (
    <img alt="" src={src} width={54} height={63} style={markFrameStyle} />
  );
}

const markFrameStyle: CSSProperties = {
  width: 54,
  height: 63,
};
