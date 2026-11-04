import { describe, expect, it } from "vitest";
import tools from "~/generated/tools.json";
import { loop } from "./cards";
import { prompts } from "./prompts";

const names = new Set(tools.families.flatMap((family) => family.tools.map((tool) => tool.name)));

describe("content names real tools", () => {
  it("every prompt's tools exist", () => {
    for (const prompt of prompts) for (const tool of prompt.tools) expect(names, tool).toContain(tool);
  });

  it("every step of the loop names a tool that exists", () => {
    for (const step of loop) expect(names, step.tool).toContain(step.tool);
  });
});
