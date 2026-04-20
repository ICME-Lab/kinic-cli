import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { OGP_FRAME_SRC, OGP_LOGO_SRC } from "./ogp-assets";
import { renderOgpImage } from "./ogp-image";

describe("renderOgpImage", () => {
  it("uses portal branding for the root card", () => {
    const markup = renderToStaticMarkup(renderOgpImage({}));

    expect(markup).toContain("Kinic Portal");
    expect(markup).toContain("KinicMemory");
    expect(markup).toContain("Share public Kinic memory canisters over the web");
    expect(markup).toContain("NETWORK");
    expect(markup).toContain("IC Mainnet");
    expect(markup).toContain("VISIBILITY");
    expect(markup).toContain("Public");
    expect(markup).toContain(
      "Kinic is your cryptographically secure, searchable memory for AI",
    );
    expect(markup).not.toContain("STATUS");
    expect(markup).not.toContain("ASPECT RATIO");
    expect(markup).not.toContain(">Shared notes and context from Kinic<");
    expect(markup).toContain(escapeMarkup(OGP_FRAME_SRC));
    expect(markup).toContain(escapeMarkup(OGP_LOGO_SRC));
  });

  it("uses memory card copy when a memory payload is present", () => {
    const markup = renderToStaticMarkup(
      renderOgpImage({
        memory: {
          memoryId: "ywega-gaaaa-aaaak-apg6q-cai",
          name: "Skill Store",
          description: "Shared notes",
          owner: "rdmx6-jaaaa-aaaaa-aaadq-cai",
        },
      }),
    );

    expect(markup).toContain("Skill Store");
    expect(markup).toContain("Shared notes");
    expect(markup).toContain("MEMORY ID");
    expect(markup).toContain("ywega-gaaaa-aaaak-apg6q-cai");
    expect(markup).toContain("OWNER");
    expect(markup).toContain("rdmx6...cai");
    expect(markup).toContain("IC Mainnet");
    expect(markup).toContain("Public");
    expect(markup).toContain("Kinic is your cryptographically secure");
  });
});

function escapeMarkup(value: string): string {
  return value.replaceAll("'", "&#x27;");
}
