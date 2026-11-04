import { describe, expect, it } from "vitest";
import recording from "../../../public/sessions/image.claude-code.json";
import { demoSteps, validateDemo } from "./Demo";
import type { Recording } from "../client/types";

describe("guided demo", () => {
  it("has six steps backed by the real image-to-Figma recording", () => {
    expect(demoSteps).toHaveLength(6);
    expect(validateDemo(recording as Recording)).toEqual([]);
  });
});
