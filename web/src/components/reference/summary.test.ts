import { describe, expect, it } from "vitest";
import tools from "~/generated/tools.json";
import { SUMMARY_MAX, summary } from "./summary";

const all = tools.families.flatMap((family) => family.tools);

describe("a tool's line when closed", () => {
  it("is the first sentence when it is short", () => {
    expect(summary("Close an open file. Its handle stops working.")).toBe("Close an open file.");
    expect(summary("Make a collection, e.g. Light and Dark. Then more.")).toBe(
      "Make a collection, e.g. Light and Dark.",
    );
  });

  it("cuts a long first sentence at its colon, or at a word", () => {
    const long = `Read one node exactly, in Figma's terms: ${"geometry, fills, ".repeat(10)}and more.`;
    expect(summary(long)).toBe("Read one node exactly, in Figma's terms");
    expect(summary("word ".repeat(40).trim() + ".")).toMatch(/^word( word)*…$/);
  });

  it("is, for every tool, short and the server's own words", () => {
    for (const tool of all) {
      const line = summary(tool.description);
      expect(line.length, tool.name).toBeGreaterThan(10);
      expect(line.length, tool.name).toBeLessThanOrEqual(SUMMARY_MAX + 1);
      expect(tool.description.startsWith(line.replace(/…$/, "")), tool.name).toBe(true);
    }
  });
});
