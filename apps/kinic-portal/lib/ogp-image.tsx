// Where: shared by the default and memory-specific OGP image routes.
// What: renders the dark-framed Kinic social card with static SVG assets and bounded copy.
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

const CANVAS_WIDTH = 1200;
const CANVAS_HEIGHT = 630;
const INNER_INSET = 18;
const PANEL_WIDTH = CANVAS_WIDTH - INNER_INSET * 2;
const PANEL_HEIGHT = CANVAS_HEIGHT - INNER_INSET * 2;
const FOOTER_TAGLINE =
  "Kinic is your cryptographically secure, searchable memory for AI — every bookmark, email, note, and document in one place.";

export function renderOgpImage({ memory }: OgpImageProps): ReactElement {
  const card = memory ? buildMemoryOgpCardModel(memory) : { title: DEFAULT_SITE_TITLE, description: DEFAULT_SITE_DESCRIPTION, shortMemoryId: "-", owner: null };
  return (
    <div style={frameStyle}>
      <div style={outerCardStyle}>
        <div style={innerPanelStyle}>
          <img
            alt=""
            src={OGP_FRAME_SRC}
            width={PANEL_WIDTH}
            height={PANEL_HEIGHT}
            style={backgroundImageStyle}
          />
          <div style={panelStyle}>
            <div style={headerStyle}>
              <div style={brandStyle}>
                <KinicMark src={OGP_LOGO_SRC} />
                <div style={brandTextStyle}>KinicMemory</div>
              </div>
              <div style={metaGridStyle}>
                <MetaStat label="NETWORK" value="IC Mainnet" />
                <MetaStat label="VISIBILITY" value="Public" />
                {card.owner ? <MetaStat label="OWNER" value={shortenOwner(card.owner)} /> : null}
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
            <div style={footerStyle}>
              <div style={footerLineStyle} />
              <div style={footerTextStyle}>{FOOTER_TAGLINE}</div>
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

function MetaStat({ label, value }: { label: string; value: string }) {
  return (
    <div style={metaStatStyle}>
      <div style={metaLabelStyle}>{label}</div>
      <div style={metaValueStyle}>{value}</div>
    </div>
  );
}

function shortenOwner(value: string): string {
  if (value.length <= 9) {
    return value;
  }
  return `${value.slice(0, 5)}...${value.slice(-3)}`;
}

const frameStyle: CSSProperties = {
  width: "100%",
  height: "100%",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  position: "relative",
  background: "#060c1b",
  color: "#f7fbff",
  fontFamily: "system-ui, sans-serif",
  overflow: "hidden",
};

const outerCardStyle: CSSProperties = {
  width: CANVAS_WIDTH,
  height: CANVAS_HEIGHT,
  display: "flex",
  position: "relative",
  overflow: "hidden",
  borderRadius: 24,
  border: "1px solid rgba(214, 226, 246, 0.16)",
  background: "rgba(10, 16, 31, 0.86)",
};

const innerPanelStyle: CSSProperties = {
  margin: INNER_INSET,
  width: PANEL_WIDTH,
  height: PANEL_HEIGHT,
  display: "flex",
  position: "relative",
  overflow: "hidden",
  borderRadius: 16,
  border: "1px solid rgba(188, 205, 232, 0.08)",
  background: "rgba(9, 15, 29, 0.84)",
};

const backgroundImageStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  width: "100%",
  height: "100%",
};

const panelStyle: CSSProperties = {
  margin: 0,
  padding: "36px 42px 40px",
  width: "100%",
  height: "100%",
  display: "flex",
  flexDirection: "column",
  gap: 28,
  justifyContent: "space-between",
  position: "relative",
};

const headerStyle: CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "space-between",
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

const metaGridStyle: CSSProperties = {
  display: "flex",
  gap: 18,
  alignItems: "flex-start",
};

const metaStatStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 6,
  minWidth: 112,
  maxWidth: 168,
};

const metaLabelStyle: CSSProperties = {
  fontSize: 12,
  lineHeight: 1,
  letterSpacing: "0.12em",
  textTransform: "uppercase",
  color: "#8da2c5",
};

const metaValueStyle: CSSProperties = {
  display: "-webkit-box",
  overflow: "hidden",
  fontSize: 24,
  lineHeight: 1.1,
  fontWeight: 600,
  color: "#eef4ff",
  WebkitBoxOrient: "vertical",
  WebkitLineClamp: 2,
};

const bodyStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 20,
  width: 900,
};

const heroStyle: CSSProperties = {
  display: "flex",
  gap: 18,
  alignItems: "flex-start",
  marginTop: 2,
  flex: 1,
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
  width: 900,
  fontSize: 27,
  lineHeight: 1.32,
  color: "rgba(235, 241, 252, 0.92)",
  WebkitBoxOrient: "vertical",
  WebkitLineClamp: 3,
};

const statsGridStyle: CSSProperties = {
  display: "flex",
  gap: 16,
  width: 900,
  marginTop: 10,
};

const footerStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 18,
  width: "100%",
  paddingBottom: 6,
};

const footerLineStyle: CSSProperties = {
  width: "100%",
  height: 0,
  borderTop: "1px solid rgba(245, 247, 250, 0.92)",
};

const footerTextStyle: CSSProperties = {
  fontSize: 17,
  lineHeight: 1.35,
  color: "rgba(245, 247, 250, 0.94)",
  textAlign: "center",
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
