/** One scene of the home's video, as its captions file names and times it. */
export interface Scene {
  name: string;
  text: string;
  /** In seconds. */
  start: number;
  end: number;
}

const TIME = /^(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/;

function seconds(stamp: string): number {
  const match = TIME.exec(stamp.trim());
  if (!match) throw new Error(`vtt: "${stamp}" is not HH:MM:SS.mmm`);
  const [, h, m, s, ms] = match.map(Number) as [number, number, number, number, number];
  return h * 3600 + m * 60 + s + ms / 1000;
}

/**
 * The scenes of a WebVTT file written by grafta:video/scripts/vtt.mjs: a cue per
 * scene, its identifier the name, end to end from 0 (internal: specs/103,
 * contracts/scenes-vtt.md). Throws on a file that breaks those rules.
 */
export function readScenes(source: string): Scene[] {
  const blocks = source
    .replace(/\r\n/g, "\n")
    .trim()
    .split(/\n{2,}/);
  if (blocks.shift()?.trim() !== "WEBVTT") throw new Error("vtt: the file does not start with WEBVTT");
  const scenes = blocks.map((block) => {
    const [name, times, ...text] = block.split("\n");
    const [from, to] = (times ?? "").split("-->");
    if (!name || !from || !to || text.length === 0)
      throw new Error(`vtt: a cue is not name, times, text: "${block}"`);
    return { name: name.trim(), text: text.join(" ").trim(), start: seconds(from), end: seconds(to) };
  });
  scenes.forEach((scene, i) => {
    const before = scenes[i - 1]?.end ?? 0;
    if (Math.abs(scene.start - before) > 0.001)
      throw new Error(`vtt: "${scene.name}" does not start where the scene before ends`);
    if (scene.end <= scene.start) throw new Error(`vtt: "${scene.name}" ends before it starts`);
  });
  if (scenes.length === 0) throw new Error("vtt: no cues");
  return scenes;
}
