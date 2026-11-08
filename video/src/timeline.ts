// The scene's timeline, in frames at 30 per second, free of Remotion so the
// scripts that write its captions can read it too.
import { EXCHANGES } from "./talk";

export const FPS = 30;

/** The timeline, in frames at 30 fps. */
export const T = {
  click: 104,
  open: 112,
  dragFrom: 150,
  dragTo: 192,
  types: [206, 436, 696],
  vscode: 870,
  figma: 1110,
  end: 1310,
  total: 1450,
};
export const sendOf = (k: number) => (T.types[k] ?? 0) + (EXCHANGES[k]?.words.length ?? 0) + 12;
export const STEP_GAP = 26;
export const stepAt = (k: number, i: number) => sendOf(k) + 16 + i * STEP_GAP;
export const replyAt = (k: number) => stepAt(k, EXCHANGES[k]?.steps.length ?? 0) + 4;
export const cardAt = () => replyAt(2) + 24;
export const openCodeAt = () => T.vscode - 14;
/** The double click on order-light.fig, and when Figma has it open. */
export const openFigAt = () => T.figma - 12;
export const figmaReady = () => T.figma + 30;

