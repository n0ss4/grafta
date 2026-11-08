import { describe, expect, it } from "vitest";
import { activeAt, progressAt, shares } from "./rail";

const scenes = [
  { name: "A", text: "a", start: 0, end: 4 },
  { name: "B", text: "b", start: 4, end: 10 },
  { name: "C", text: "c", start: 10, end: 20 },
];

describe("the scene rail", () => {
  it("marks the scene whose start the time has reached", () => {
    expect(activeAt(scenes, 0)).toBe(0);
    expect(activeAt(scenes, 3.99)).toBe(0);
    expect(activeAt(scenes, 4)).toBe(1);
    expect(activeAt(scenes, 25)).toBe(2);
  });

  it("fills a scene as it plays, and no further", () => {
    expect(progressAt(scenes, 7, 1)).toBe(0.5);
    expect(progressAt(scenes, 2, 1)).toBe(0);
    expect(progressAt(scenes, 12, 1)).toBe(1);
  });

  it("gives each scene its share of the time", () => {
    expect(shares(scenes)).toEqual([0.2, 0.3, 0.5]);
  });
});
