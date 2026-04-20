import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  renderOgpImage: vi.fn(),
  imageResponse: vi.fn(),
}));

vi.mock("@/lib/ogp-image", () => ({
  renderOgpImage: mocks.renderOgpImage,
}));

vi.mock("next/og", () => ({
  ImageResponse: class extends Response {
    constructor(markup: unknown, options: unknown) {
      mocks.imageResponse(markup, options);
      super("png", {
        headers: {
          "content-type": "image/png",
        },
      });
    }
  },
}));

import { GET } from "./route";

describe("memory og api route", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.renderOgpImage.mockReturnValue("markup");
  });

  it("renders using query-provided memory copy", async () => {
    const response = await GET(
      new Request("https://portal.kinic.io/api/og/memories/m1?name=Skill%20Store&description=cached%20summary"),
      {
        params: Promise.resolve({ memoryId: "m1" }),
      },
    );

    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Skill Store",
        description: "cached summary",
      },
    });
    expect(mocks.imageResponse).toHaveBeenCalledWith("markup", { width: 1200, height: 630 });
    expect(response.headers.get("content-type")).toBe("image/png");
    expect(response.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
  });

  it("normalizes blank query params to null", async () => {
    await GET(
      new Request("https://portal.kinic.io/api/og/memories/m1?name=%20%20%20&description=%0A"),
      {
        params: Promise.resolve({ memoryId: "m1" }),
      },
    );

    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: null,
        description: null,
      },
    });
  });
});
