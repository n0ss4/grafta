// The six scenes of the video, end to end: what the page's rail names, what its
// captions track says, and what the shared file shows in its picture.
import { FPS, T, openFigAt, sendOf } from "./timeline";

export interface Scene {
  /** Short, the rail's label and the cue's identifier. */
  name: string;
  /** One sentence: the caption, and what a screen reader reads. */
  text: string;
  /** In frames. */
  from: number;
  to: number;
  /** Whether the shared file shows the text in its picture. */
  burn: boolean;
}

const bounds = [0, T.dragFrom - 10, sendOf(0) + 10, sendOf(1), sendOf(2), openFigAt() - 20, T.total];

const words: [string, string, boolean][] = [
  ["Your AI app", "Talk to your Figma files, from the AI app you already use.", false],
  ["Add a file", "Get a Figma file, and add it to the chat.", true],
  ["Ask what's in it", "Ask what's in it: grafta reads its layers, variables and a picture.", true],
  ["Change it, check it", "Change it, and check it. It saves a new file, never over yours.", true],
  ["Turn it into code", "Turn it into code, in whatever language you work in.", true],
  ["Open it in Figma", "Open the new file in Figma: real layers and variables, to keep editing.", true],
];

export const SCENES: Scene[] = words.map(([name, text, burn], i) => ({
  name,
  text,
  burn,
  from: bounds[i]!,
  to: bounds[i + 1]!,
}));

/** A frame as WebVTT's HH:MM:SS.mmm. */
export function timestamp(frame: number): string {
  const ms = Math.round((frame / FPS) * 1000);
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(Math.floor(ms / 3_600_000))}:${pad(Math.floor(ms / 60_000) % 60)}:${pad(Math.floor(ms / 1000) % 60)}.${pad(ms % 1000, 3)}`;
}

/** The scenes as WebVTT, per internal:specs/103-video-first-screen/contracts/scenes-vtt.md. */
export function vtt(scenes: Scene[] = SCENES): string {
  const cues = scenes.map((s) => `${s.name}\n${timestamp(s.from)} --> ${timestamp(s.to)}\n${s.text}`);
  return `WEBVTT\n\n${cues.join("\n\n")}\n`;
}
