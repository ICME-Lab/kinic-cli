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
    expect(markup).not.toContain("Shared Memory</div>");
    expect(markup).toContain(OGP_FRAME_SRC);
    expect(markup).toContain(OGP_LOGO_SRC);
  });

  it("uses memory card copy when a memory payload is present", () => {
    const markup = renderToStaticMarkup(
      renderOgpImage({
        memory: {
          memoryId: "ywega-gaaaa-aaaak-apg6q-cai",
          name: "Skill Store",
          description: "Shared notes",
        },
      }),
    );

    expect(markup).toContain("Skill Store");
    expect(markup).toContain("Shared notes");
    expect(markup).toContain("MEMORY ID");
    expect(markup).toContain("ywega-gaaaa-aaaak-apg6q-cai");
  });
});
