import { describe, expect, it } from "vitest";
import tools from "~/generated/tools.json";
import { filterFamilies } from "./filter";

const names = (query: string) =>
  filterFamilies(tools.families, query).flatMap((f) => f.tools.map((t) => t.name));

describe("filterFamilies", () => {
  it("keeps every tool for an empty query", () => {
    expect(names("")).toHaveLength(tools.count);
  });

  it("matches a tool's name and its arguments' names", () => {
    expect(names("fill")).toContain("fill_template");
    expect(names("fill")).toContain("create_text"); // `fills`
    expect(names("zzz")).toHaveLength(0);
  });
});
