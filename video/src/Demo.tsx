import { AbsoluteFill, Easing, interpolate, useCurrentFrame } from "remotion";
import { CARD, CarriedFile, ClaudeWindow, DesktopFile, INPUT, WIN, carriedAt } from "./Claude";
import { DOCK, Dock, FILE, dockX } from "./Dock";
import { EndCard } from "./End";
import { FIG, FigmaWindow } from "./Figma";
import { RECORDED } from "./talk";
import { C, SANS, T, clamp, figmaReady, openCodeAt, openFigAt, ramp, sendOf } from "./shared";
import { FIG_ROW, VS, VSCodeWindow } from "./VSCode";

// One scene, drawn from the frame number, so the same component plays on the
// page and renders to a file: the apps, Claude Desktop with grafta, the code
// in VS Code, the result in Figma.

export { FPS, HEIGHT, WIDTH } from "./shared";
export const DURATION = T.total;

export function Demo() {
  return (
    <AbsoluteFill style={{ background: C.bg, fontFamily: SANS, color: C.ink }}>
      <Dock />
      <DesktopFile />
      <ClaudeWindow />
      <CarriedFile />
      <VSCodeWindow />
      <FigmaWindow />
      <Captions />
      <EndCard />
      <Cursor />
    </AbsoluteFill>
  );
}

// ——— Words under the picture ———

const CAPTIONS: { from: number; to: number; text: string }[] = [
  { from: T.dragFrom - 10, to: sendOf(0) + 10, text: "Get a Figma file, and add it to the chat." },
  { from: sendOf(0) + 10, to: sendOf(1), text: "Ask what's in it." },
  { from: sendOf(1), to: sendOf(2), text: "Change it, and check it. It saves a new file, never over yours." },
  { from: sendOf(2), to: openFigAt() - 20, text: "Turn it into code, in whatever language you work in." },
  { from: openFigAt() - 20, to: T.end, text: "Open the new file in Figma: real layers and variables, to keep editing." },
];

function Captions() {
  const frame = useCurrentFrame();
  const cap = CAPTIONS.find((c) => frame >= c.from && frame < c.to);
  const footnote = frame >= T.open + 20 && frame < T.figma ? ramp(frame, T.open + 20, T.open + 34) : 0;
  return (
    <>
      {cap ? (
        <div
          key={cap.text}
          style={{
            position: "absolute",
            top: 930,
            width: "100%",
            textAlign: "center",
            fontSize: 38,
            fontWeight: 600,
            letterSpacing: "-0.02em",
            opacity: ramp(frame, cap.from, cap.from + 10) * (1 - ramp(frame, cap.to - 8, cap.to)),
          }}
        >
          {cap.text}
        </div>
      ) : null}
      <div style={{ position: "absolute", bottom: 22, right: 40, fontSize: 15, color: C.faint, opacity: footnote }}>
        Requests, replies and code from a real session, shortened: {RECORDED}
      </div>
    </>
  );
}

// ——— The pointer ———

const PATH: [number, number, number][] = [
  [0, 1180, 900],
  [60, 1180, 900],
  [T.click - 4, dockX(0) + 10, DOCK.y + 70],
  [T.click + 30, 700, 700],
  [T.dragFrom - 6, FILE.x + 6, FILE.y + 6],
  [T.dragFrom, FILE.x + 6, FILE.y + 6],
  [T.dragTo, INPUT.x + 36, WIN.y + 406],
  [T.dragTo + 30, 1500, 980],
  [openCodeAt() - 30, 1500, 980],
  [openCodeAt() - 2, CARD.x, CARD.y],
  [T.vscode + 30, CARD.x + 100, CARD.y + 80],
  [T.vscode + 110, VS.x + 1350, VS.y + 400],
  [openFigAt() - 50, VS.x + 1350, VS.y + 400],
  [openFigAt() - 4, FIG_ROW.x, FIG_ROW.y],
  [openFigAt() + 8, FIG_ROW.x, FIG_ROW.y],
  [figmaReady() + 30, FIG.x + 900, FIG.y + 560],
  [figmaReady() + 60, FIG.x + 760, FIG.y + 300],
  [T.end, FIG.x + 1100, FIG.y + 600],
];
const CLICKS = [T.click, T.dragFrom, openCodeAt(), openFigAt() - 6, openFigAt(), figmaReady() + 62];

function Cursor() {
  const frame = useCurrentFrame();
  if (frame >= T.end) return null;
  const fs = PATH.map((p) => p[0]);
  let x = interpolate(frame, fs, PATH.map((p) => p[1]), { ...clamp, easing: Easing.inOut(Easing.cubic) });
  let y = interpolate(frame, fs, PATH.map((p) => p[2]), { ...clamp, easing: Easing.inOut(Easing.cubic) });
  if (frame >= T.dragFrom && frame <= T.dragTo) {
    // holding the file: the pointer moves with it
    const held = carriedAt(frame);
    x = held.x + 6;
    y = held.y + 6;
  }
  const click = [...CLICKS].reverse().find((c) => frame >= c && frame < c + 14);
  const r = click !== undefined ? ramp(frame, click, click + 14) : 0;
  return (
    <>
      {click !== undefined ? (
        <div
          style={{
            position: "absolute",
            left: x - 24 * r - 4,
            top: y - 24 * r - 4,
            width: 8 + 48 * r,
            height: 8 + 48 * r,
            borderRadius: "50%",
            border: `3px solid ${C.sap}`,
            opacity: 1 - r,
          }}
        />
      ) : null}
      <svg
        width={30}
        height={30}
        viewBox="0 0 24 24"
        style={{ position: "absolute", left: x - 4, top: y - 2, filter: "drop-shadow(0 2px 3px rgb(0 0 0 / 0.3))" }}
      >
        <path d="M4 2 L4 19 L8.5 15 L11.5 22 L14.5 20.7 L11.6 14 L18 14 Z" fill="#111" stroke="#fff" strokeWidth={1.5} strokeLinejoin="round" />
      </svg>
    </>
  );
}
