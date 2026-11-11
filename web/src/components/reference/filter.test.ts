import { describe, expect, it } from "vitest";
import tools from "~/generated/tools.json";
import { filterFamilies } from "./filter";

const found = (query: string) => filterFamilies(tools.families, query).flatMap((f) => f.tools);
const names = (query: string) => found(query).map((f) => f.tool.name);

describe("filterFamilies", () => {
  it("keeps every tool for an empty query", () => {
    expect(names("")).toHaveLength(tools.count);
  });

  it("matches a tool's name and its arguments' names", () => {
    expect(names("fill")).toContain("fill_template");
    expect(names("fill")).toContain("create_text"); // `fills`
    expect(names("zzz")).toHaveLength(0);
  });

  it("finds tools by the job, in their descriptions", () => {
    expect(names("picture")).toContain("render_node");
    expect(names("colour").length).toBeGreaterThan(0);
    expect(names("Contrast")).toContain("check_contrast");
  });

  it("says which matched only inside", () => {
    const render = found("picture").find((f) => f.tool.name === "render_node");
    expect(render?.inside).toBe(true);
    expect(found("render_node").find((f) => f.tool.name === "render_node")?.inside).toBe(false);
  });
});
