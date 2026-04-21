// Where: shared by the default and memory-specific OGP image routes.
// What: renders the Kinic social card as static chrome plus a small dynamic text overlay.
// Why: Cloudflare OGP rendering must keep runtime layout work as small as possible.

import type { CSSProperties, ReactElement } from "react";
import {
  buildMemoryOgpCardModel,
  type MemoryOgpInput,
} from "@kinic/kinic-share";
import { OGP_FRAME_SRC, OGP_LOGO_SRC, OGP_MEMORY_CHROME_SRC } from "./ogp-assets";
import {
  DEFAULT_SITE_DESCRIPTION,
  DEFAULT_SITE_TITLE,
} from "./site-metadata";

type OgpImageProps = {
  memory?: MemoryOgpInput;
};

const CANVAS_WIDTH = 1200;
const CANVAS_HEIGHT = 630;

export function renderOgpImage({ memory }: OgpImageProps): ReactElement {
  if (!memory) {
    return renderSiteOgpImage();
  }

  const card = buildMemoryOgpCardModel(memory);
  return (
    <div style={frameStyle}>
      <img
        alt=""
        src={OGP_MEMORY_CHROME_SRC}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        style={chromeImageStyle}
      />
      <div style={overlayStyle}>
        <div style={bodyStyle}>
          <div style={titleStyle}>{card.title}</div>
          <div style={descriptionStyle}>{card.description}</div>
          <div style={memoryIdRowStyle}>
            <span style={memoryIdLabelStyle}>MEMORY ID</span>
            <span style={memoryIdValueStyle}>{card.shortMemoryId}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function renderSiteOgpImage(): ReactElement {
  return (
    <div style={frameStyle}>
      <img
        alt=""
        src={OGP_FRAME_SRC}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        style={chromeImageStyle}
      />
      <div style={siteOverlayStyle}>
        <div style={siteBrandRowStyle}>
          <img alt="" src={OGP_LOGO_SRC} width={58} height={68} style={siteLogoStyle} />
          <div style={siteBrandTitleStyle}>Kinic</div>
        </div>
        <div style={siteContentStyle}>
          <div style={siteTitleStyle}>{DEFAULT_SITE_TITLE}</div>
          <div style={siteDescriptionStyle}>{DEFAULT_SITE_DESCRIPTION}</div>
        </div>
      </div>
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

const chromeImageStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  width: "100%",
  height: "100%",
};

const overlayStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  display: "flex",
};

const siteOverlayStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  display: "flex",
  flexDirection: "column",
  padding: "56px 60px 64px",
};

const siteBrandRowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 20,
};

const siteLogoStyle: CSSProperties = {
  width: 58,
  height: 68,
};

const siteBrandTitleStyle: CSSProperties = {
  fontSize: 42,
  fontWeight: 700,
  letterSpacing: "-0.04em",
  color: "#dbe9ff",
};

const siteContentStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 26,
  width: 980,
  marginTop: 80,
};

const siteTitleStyle: CSSProperties = {
  fontSize: 92,
  fontWeight: 800,
  lineHeight: 0.98,
  letterSpacing: "-0.06em",
  color: "#ffffff",
};

const siteDescriptionStyle: CSSProperties = {
  width: 880,
  fontSize: 34,
  lineHeight: 1.34,
  color: "rgba(235, 241, 252, 0.92)",
};

const bodyStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 20,
  width: 900,
  position: "absolute",
  left: 60,
  top: 158,
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

const memoryIdRowStyle: CSSProperties = {
  display: "flex",
  gap: 14,
  alignItems: "center",
  marginTop: 10,
};

const memoryIdLabelStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  lineHeight: 1,
  fontSize: 20,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "#9db4d6",
};

const memoryIdValueStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  lineHeight: 1,
  fontSize: 26,
  fontWeight: 500,
  color: "#eff4ff",
};
