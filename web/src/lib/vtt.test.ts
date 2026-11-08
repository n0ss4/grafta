import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { readScenes } from "./vtt";

const file = (cues: string) => `WEBVTT\n\n${cues}\n`;

describe("the video's scenes", () => {
  it("reads the site's captions file: six scenes, end to end", () => {
    const scenes = readScenes(readFileSync(join(process.cwd(), "public/showcase/video/demo.vtt"), "utf8"));
    expect(scenes.map((s) => s.name)).toEqual([
      "Your AI app",
      "Add a file",
      "Ask what's in it",
      "Change it, check it",
      "Turn it into code",
      "Open it in Figma",
    ]);
    expect(scenes[0]?.start).toBe(0);
  });

  it("reads a cue's name, times and text", () => {
    expect(readScenes(file("One\n00:00:00.000 --> 00:01:02.500\nThe first."))).toEqual([
      { name: "One", text: "The first.", start: 0, end: 62.5 },
    ]);
  });

  it("refuses a gap between scenes", () => {
    expect(() =>
      readScenes(file("A\n00:00:00.000 --> 00:00:01.000\na\n\nB\n00:00:02.000 --> 00:00:03.000\nb")),
    ).toThrow(/does not start where/);
  });

  it("refuses a file that is not WebVTT", () => {
    expect(() => readScenes("A\n00:00:00.000 --> 00:00:01.000\na")).toThrow(/WEBVTT/);
  });
});
