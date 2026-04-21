import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { OGP_FRAME_SRC, OGP_LOGO_SRC, OGP_MEMORY_CHROME_SRC } from "./ogp-assets";
import { renderOgpImage } from "./ogp-image";

describe("renderOgpImage", () => {
  it("uses portal branding for the root card", () => {
    const markup = renderToStaticMarkup(renderOgpImage({}));

    expect(markup).toContain("Kinic Portal");
    expect(markup).toContain("Share public Kinic knowledge over the web");
    expect(markup).toContain(">Kinic<");
    expect(markup).not.toContain("MEMORY ID");
    expect(markup).not.toContain("KinicMemory");
    expect(markup).toContain(
      escapeMarkup(OGP_FRAME_SRC),
    );
    expect(markup).toContain(escapeMarkup(OGP_LOGO_SRC));
    expect(markup).not.toContain("STATUS");
    expect(markup).not.toContain("ASPECT RATIO");
    expect(markup).not.toContain(">Shared notes and context from Kinic<");
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
    expect(markup).not.toContain("rdmx6...cai");
    expect(markup).not.toContain("OWNER");
    expect(markup).toContain(escapeMarkup(OGP_MEMORY_CHROME_SRC));
  });
});

function escapeMarkup(value: string): string {
  return value.replaceAll("'", "&#x27;");
}
