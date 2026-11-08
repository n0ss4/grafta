import type { Scene } from "~/lib/vtt";

/** The scene playing at a time: the last whose start it has reached. */
export function activeAt(scenes: Scene[], time: number): number {
  let active = 0;
  scenes.forEach((scene, i) => {
    if (time >= scene.start) active = i;
  });
  return active;
}

/** How much of scene `i` has played at a time, from 0 to 1. */
export function progressAt(scenes: Scene[], time: number, i: number): number {
  const scene = scenes[i];
  if (!scene) return 0;
  return Math.min(1, Math.max(0, (time - scene.start) / (scene.end - scene.start)));
}

/** Each scene's share of the running time, for the rail's widths. */
export function shares(scenes: Scene[]): number[] {
  const total = scenes.reduce((sum, s) => sum + (s.end - s.start), 0);
  return scenes.map((s) => (s.end - s.start) / total);
}
