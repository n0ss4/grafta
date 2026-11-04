import { describe, expect, it } from "vitest";
import { CARD, meta } from "./seo";

describe("meta", () => {
  const tags = meta({ title: "T", description: "D", path: "/tools/" });

  it("names the card by an absolute address", () => {
    expect(CARD).toBe("https://n0ss4.github.io/grafta/assets/card.png");
    expect(tags).toContainEqual({ property: "og:image", content: CARD });
    expect(tags).toContainEqual({ name: "twitter:image", content: CARD });
  });

  it("gives the canonical address under the site", () => {
    expect(tags).toContainEqual({
      tagName: "link",
      rel: "canonical",
      href: "https://n0ss4.github.io/grafta/tools/",
    });
  });
});
