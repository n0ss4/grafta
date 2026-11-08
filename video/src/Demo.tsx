import "@fontsource-variable/inter";
import type { CSSProperties, ReactNode } from "react";
import { AbsoluteFill, Easing, Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { APPS, CODE, CONTRAST, EXCHANGES, LAYERS, RECORDED } from "./talk";

// One scene, timed in frames at 30 fps. Everything is drawn from the frame
// number, so the same component plays on the page and renders to a file.

export const FPS = 30;
export const WIDTH = 1920;
export const HEIGHT = 1080;

const C = {
  bg: "#E8E8E4",
  surface: "#FFFFFF",
  sunk: "#F2F2EE",
  ink: "#0F1115",
  muted: "#4A4E55",
  faint: "#8A8E95",
  line: "#D9DAD5",
  sap: "#1F9D6B",
  deep: "#13704B",
  bud: "#C6F26B",
  soft: "#E3F4EA",
  bubble: "#ECEAE3",
  select: "#0D99FF",
};
const SANS = "'Inter Variable', Inter, system-ui, sans-serif";
const MONO = "ui-monospace, 'SF Mono', Menlo, monospace";

// The timeline.
const T = {
  click: 104,
  open: 112,
  dragFrom: 150,
  dragTo: 192,
  types: [206, 436, 688],
  figma: 900,
  end: 1068,
  total: 1200,
};
const sendOf = (k: number) => (T.types[k] ?? 0) + (EXCHANGES[k]?.words.length ?? 0) + 12;
const STEP_GAP = 30;
const stepAt = (k: number, i: number) => sendOf(k) + 14 + i * STEP_GAP;
const replyAt = (k: number) => stepAt(k, EXCHANGES[k]?.steps.length ?? 0) + 6;

export const DURATION = T.total;

// Layout: the chat window, and where things sit on screen.
const WIN = { x: 250, y: 64, w: 1420, h: 820 };
const DOCK = { y: 560, tile: 132, gap: 40 };
const dockX = (i: number) => {
  const width = APPS.length * DOCK.tile + (APPS.length - 1) * DOCK.gap;
  return (WIDTH - width) / 2 + i * (DOCK.tile + DOCK.gap) + DOCK.tile / 2;
};
const FILE = { x: 118, y: 470 };
const INPUT = { x: WIN.x + 150, y: WIN.y + WIN.h - 62 };

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;
const ease = Easing.bezier(0.25, 0.1, 0.25, 1);
const ramp = (f: number, from: number, to: number) => interpolate(f, [from, to], [0, 1], { ...clamp, easing: ease });

function useSpring(at: number, damping = 18) {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return spring({ frame: frame - at, fps, config: { damping, mass: 0.7 } });
}

/** Fades and lifts in at a frame. */
function Appear({ at, children, style, dy = 14 }: { at: number; children: ReactNode; style?: CSSProperties; dy?: number }) {
  const frame = useCurrentFrame();
  const p = ramp(frame, at, at + 10);
  return <div style={{ opacity: p, transform: `translateY(${(1 - p) * dy}px)`, ...style }}>{children}</div>;
}

export function Demo() {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill style={{ background: C.bg, fontFamily: SANS, color: C.ink }}>
      <Dock />
      <DesktopFile />
      <ChatWindow />
      <FigmaWindow />
      <Captions />
      <EndCard />
      <Cursor frame={frame} />
    </AbsoluteFill>
  );
}

// ——— The apps, then the click on one ———

function Dock() {
  const frame = useCurrentFrame();
  const out = ramp(frame, T.open, T.open + 14);
  if (out >= 1) return null;
  return (
    <AbsoluteFill style={{ opacity: 1 - out, transform: `scale(${1 - out * 0.06})` }}>
      <Appear at={4} style={{ position: "absolute", top: 250, width: "100%", textAlign: "center" }}>
        <div style={{ fontSize: 76, fontWeight: 650, letterSpacing: "-0.035em" }}>Talk to your Figma files.</div>
        <div style={{ fontSize: 30, color: C.muted, marginTop: 18 }}>From the AI app you already use.</div>
      </Appear>
      {APPS.map((app, i) => (
        <AppTile key={app.name} i={i} />
      ))}
    </AbsoluteFill>
  );
}

function AppTile({ i }: { i: number }) {
  const frame = useCurrentFrame();
  const app = APPS[i]!;
  const pop = useSpring(18 + i * 5, 12);
  const chosen = i === 0;
  const press = chosen ? 1 - 0.08 * Math.sin(Math.PI * ramp(frame, T.click - 2, T.click + 6)) : 1;
  const dim = chosen ? 0 : ramp(frame, T.click, T.click + 8) * 0.55;
  return (
    <div
      style={{
        position: "absolute",
        left: dockX(i) - DOCK.tile / 2,
        top: DOCK.y,
        width: DOCK.tile,
        textAlign: "center",
        opacity: 1 - dim,
        transform: `translateY(${(1 - pop) * 40}px) scale(${pop * press})`,
      }}
    >
      <div
        style={{
          width: DOCK.tile,
          height: DOCK.tile,
          borderRadius: 30,
          background: app.tint,
          color: "#fff",
          display: "grid",
          placeItems: "center",
          fontSize: 44,
          fontWeight: 650,
          fontFamily: app.short === ">_" ? MONO : SANS,
          boxShadow: "0 10px 30px rgb(15 17 21 / 0.18), inset 0 1px 0 rgb(255 255 255 / 0.15)",
        }}
      >
        {app.short}
      </div>
      <div style={{ marginTop: 16, fontSize: 20, color: C.muted, whiteSpace: "nowrap", marginLeft: -40, marginRight: -40 }}>
        {app.name}
      </div>
    </div>
  );
}

// ——— The file on the desktop, dragged into the chat ———

function FigFile({ size = 1 }: { size?: number }) {
  return (
    <div style={{ width: 84 * size, textAlign: "center" }}>
      <div
        style={{
          width: 70 * size,
          height: 88 * size,
          margin: "0 auto",
          borderRadius: 10 * size,
          background: "#fff",
          boxShadow: "0 6px 18px rgb(15 17 21 / 0.18)",
          display: "grid",
          placeItems: "center",
        }}
      >
        <FigGlyph size={34 * size} />
      </div>
      <div style={{ marginTop: 10 * size, fontSize: 17 * size, fontWeight: 550 }}>order.fig</div>
    </div>
  );
}

/** A plain mark for a design file: a page with a folded corner, no brand. */
function FigGlyph({ size }: { size: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24">
      <path d="M6 2h8l5 5v13a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" fill={C.soft} stroke={C.deep} strokeWidth={1.6} />
      <path d="M14 2v5h5" fill="none" stroke={C.deep} strokeWidth={1.6} strokeLinejoin="round" />
      <rect x={7.5} y={11} width={4} height={4} rx={1} fill={C.deep} />
      <circle cx={14.5} cy={13} r={2} fill={C.sap} />
      <path d="M7.5 18h9" stroke={C.deep} strokeWidth={1.6} strokeLinecap="round" />
    </svg>
  );
}

function DesktopFile() {
  const frame = useCurrentFrame();
  const shown = ramp(frame, T.open + 10, T.open + 22);
  const t = ramp(frame, T.dragFrom, T.dragTo);
  const dropped = frame >= T.dragTo;
  if (frame < T.open || frame > T.figma) return null;
  const x = interpolate(t, [0, 1], [FILE.x, INPUT.x + 40]);
  const y = interpolate(t, [0, 1], [FILE.y, INPUT.y - 10]) - Math.sin(Math.PI * t) * 80;
  return (
    <>
      {/* where it was, faded while it is carried */}
      <div style={{ position: "absolute", left: FILE.x - 42, top: FILE.y - 44, opacity: shown * (t > 0 ? 0.35 : 1) * (1 - ramp(frame, T.dragTo, T.dragTo + 10)) }}>
        <FigFile />
      </div>
      {!dropped && t > 0 ? (
        <div
          style={{
            position: "absolute",
            left: x - 42,
            top: y - 44,
            transform: `scale(${1 - t * 0.35}) rotate(${Math.sin(Math.PI * t) * -6}deg)`,
            opacity: 0.95,
          }}
        >
          <FigFile />
        </div>
      ) : null}
    </>
  );
}

// ——— The chat ———

function ChatWindow() {
  const frame = useCurrentFrame();
  const open = useSpring(T.open, 16);
  const back = ramp(frame, T.figma, T.figma + 18);
  const gone = ramp(frame, T.end, T.end + 12);
  if (frame < T.open || frame > T.figma + 30) return null;
  // grows from the tile that was clicked
  const fromX = dockX(0) - (WIN.x + WIN.w / 2);
  const fromY = DOCK.y + DOCK.tile / 2 - (WIN.y + WIN.h / 2);
  return (
    <div
      style={{
        position: "absolute",
        left: WIN.x,
        top: WIN.y,
        width: WIN.w,
        height: WIN.h,
        borderRadius: 18,
        background: C.surface,
        overflow: "hidden",
        boxShadow: "0 30px 80px rgb(15 17 21 / 0.22), 0 0 0 1px rgb(15 17 21 / 0.08)",
        opacity: Math.min(1, open * 1.6) * (1 - gone),
        transform: `translate(${(1 - open) * fromX - back * 120}px, ${(1 - open) * fromY - back * 20}px) scale(${
          (0.12 + 0.88 * open) * (1 - back * 0.1)
        })`,
        filter: `brightness(${1 - back * 0.06})`,
      }}
    >
      <TitleBar title="New chat" right={<ToolChip />} />
      <div style={{ position: "absolute", top: 52, bottom: 110, left: 0, right: 0, overflow: "hidden" }}>
        {EXCHANGES.map((_, k) => (
          <ExchangeView key={k} k={k} />
        ))}
      </div>
      <InputBar />
    </div>
  );
}

function TitleBar({ title, right, dark }: { title: string; right?: ReactNode; dark?: boolean }) {
  return (
    <div
      style={{
        height: 52,
        display: "flex",
        alignItems: "center",
        padding: "0 20px",
        borderBottom: `1px solid ${dark ? "#3A3A3A" : C.line}`,
        background: dark ? "#2C2C2C" : C.surface,
        color: dark ? "#fff" : C.ink,
        position: "relative",
      }}
    >
      <div style={{ display: "flex", gap: 9 }}>
        {["#FF5F57", "#FEBC2E", "#28C840"].map((c) => (
          <div key={c} style={{ width: 13, height: 13, borderRadius: 7, background: c }} />
        ))}
      </div>
      <div style={{ position: "absolute", left: 0, right: 0, textAlign: "center", fontSize: 17, fontWeight: 550 }}>{title}</div>
      <div style={{ marginLeft: "auto", position: "relative" }}>{right}</div>
    </div>
  );
}

function ToolChip() {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        fontSize: 15,
        padding: "6px 12px",
        borderRadius: 999,
        background: C.soft,
        color: C.deep,
        fontWeight: 600,
      }}
    >
      <Mark size={16} color={C.deep} /> grafta connected
    </div>
  );
}

/** grafta's mark: two nodes and a line, from web/public/brand/mark.svg. */
function Mark({ size, color = C.ink }: { size: number; color?: string }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64">
      <path d="M14 46 L32 18 L50 46" fill="none" stroke={color} strokeWidth={6} strokeLinecap="round" strokeLinejoin="round" />
      <circle cx={14} cy={46} r={8} fill={color} />
      <circle cx={32} cy={18} r={8} fill={color} />
      <circle cx={50} cy={46} r={8} fill={C.sap} />
    </svg>
  );
}

function InputBar() {
  const frame = useCurrentFrame();
  const attached = frame >= T.dragTo && frame < sendOf(0);
  const attachPop = useSpring(T.dragTo, 12);
  // what is typed now, and whether it was just sent
  let text = "";
  for (let k = 0; k < EXCHANGES.length; k++) {
    const start = T.types[k] ?? 0;
    const words = EXCHANGES[k]!.words;
    if (frame >= start && frame < sendOf(k)) text = words.slice(0, Math.max(0, Math.floor(frame - start)));
  }
  const caret = Math.floor(frame / 15) % 2 === 0;
  const hover = frame >= T.dragFrom + 20 && frame < T.dragTo;
  return (
    <div style={{ position: "absolute", left: 120, right: 120, bottom: 26 }}>
      <div
        style={{
          minHeight: 64,
          borderRadius: 18,
          border: `${hover ? 2 : 1}px solid ${hover ? C.sap : C.line}`,
          background: hover ? C.soft : C.sunk,
          display: "flex",
          alignItems: "center",
          gap: 14,
          padding: "10px 12px 10px 18px",
        }}
      >
        {attached ? (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 10,
              background: C.surface,
              border: `1px solid ${C.line}`,
              borderRadius: 12,
              padding: "7px 12px",
              fontSize: 17,
              fontWeight: 550,
              transform: `scale(${attachPop})`,
            }}
          >
            <FigGlyph size={18} /> order.fig
          </div>
        ) : null}
        <div style={{ fontSize: 22, color: text ? C.ink : C.faint, flex: 1 }}>
          {text || (hover ? "Drop the file to add it" : "Ask anything")}
          {text && caret ? <span style={{ color: C.sap }}>|</span> : null}
        </div>
        <div
          style={{
            width: 44,
            height: 44,
            borderRadius: 22,
            background: text ? C.ink : C.line,
            color: "#fff",
            display: "grid",
            placeItems: "center",
            fontSize: 22,
          }}
        >
          ↑
        </div>
      </div>
    </div>
  );
}

function ExchangeView({ k }: { k: number }) {
  const frame = useCurrentFrame();
  const send = sendOf(k);
  const next = k + 1 < EXCHANGES.length ? sendOf(k + 1) : T.total + 100;
  if (frame < send + 7 || frame > next + 10) return null;
  const leave = ramp(frame, next, next + 9);
  const ex = EXCHANGES[k]!;
  return (
    <div
      style={{
        position: "absolute",
        inset: "26px 120px 0 120px",
        opacity: 1 - leave,
        transform: `translateY(${-leave * 160}px)`,
      }}
    >
      <Appear at={send + 7} style={{ display: "flex", justifyContent: "flex-end", alignItems: "center", gap: 12 }}>
        {k === 0 ? (
          <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 16, color: C.muted }}>
            <FigGlyph size={16} /> order.fig
          </div>
        ) : null}
        <div style={{ background: C.bubble, borderRadius: 20, padding: "14px 22px", fontSize: 22 }}>{ex.words}</div>
      </Appear>
      <div style={{ marginTop: 18, display: "flex", flexDirection: "column", gap: 8 }}>
        {ex.steps.map((step, i) => (
          <Step key={i} at={stepAt(k, i)} label={step.label} tools={step.tools} />
        ))}
      </div>
      <div style={{ marginTop: 20 }}>
        {k === 0 ? <ReplyInside at={replyAt(0)} /> : k === 1 ? <ReplyLight at={replyAt(1)} /> : <ReplyCode at={replyAt(2)} />}
      </div>
    </div>
  );
}

function Step({ at, label, tools }: { at: number; label: string; tools: string[] }) {
  const frame = useCurrentFrame();
  const done = frame >= at + STEP_GAP - 6;
  return (
    <Appear at={at} dy={6} style={{ display: "flex", alignItems: "center", gap: 12, fontSize: 18 }}>
      <div
        style={{
          width: 22,
          height: 22,
          borderRadius: 11,
          display: "grid",
          placeItems: "center",
          background: done ? C.soft : "transparent",
          border: done ? "none" : `2.5px solid ${C.line}`,
          borderTopColor: done ? undefined : C.sap,
          transform: done ? undefined : `rotate(${(frame - at) * 24}deg)`,
          color: C.deep,
          fontSize: 14,
          fontWeight: 800,
        }}
      >
        {done ? "✓" : null}
      </div>
      <Mark size={18} />
      <span style={{ fontWeight: 550 }}>grafta</span>
      <span style={{ color: C.muted }}>{label}</span>
      <span style={{ fontFamily: MONO, fontSize: 15, color: C.faint }}>{[...new Set(tools)].join(" · ")}</span>
    </Appear>
  );
}

function Lead({ at, children }: { at: number; children: ReactNode }) {
  const frame = useCurrentFrame();
  return (
    <div style={{ fontSize: 23, lineHeight: 1.5, opacity: ramp(frame, at, at + 12), maxWidth: 640 }}>{children}</div>
  );
}

function Picture({ at, src, caption, w = 300 }: { at: number; src: string; caption: string; w?: number }) {
  const pop = useSpring(at, 16);
  return (
    <div style={{ width: w, opacity: Math.min(1, pop * 1.5), transform: `translateY(${(1 - pop) * 24}px)` }}>
      <Img
        src={staticFile(src)}
        style={{ width: w, height: (w * 730) / 720, borderRadius: 14, border: `1px solid ${C.line}`, display: "block" }}
      />
      <div style={{ fontSize: 14, color: C.faint, marginTop: 8 }}>{caption}</div>
    </div>
  );
}

function ReplyInside({ at }: { at: number }) {
  return (
    <div style={{ display: "flex", gap: 56, alignItems: "flex-start" }}>
      <div style={{ flex: 1 }}>
        <Lead at={at}>{EXCHANGES[0]!.lead}</Lead>
        <Appear at={at + 20} style={{ display: "flex", gap: 10, marginTop: 22 }}>
          {["#0D1117", "#161B22", "#F6F4EE", "#9DA3AB", "#30363D", "#C6F26B", "#1C2B23", "#15171C", "#21262D"].map((c) => (
            <div key={c} style={{ width: 38, height: 38, borderRadius: 10, background: c, border: `1px solid ${C.line}` }} />
          ))}
        </Appear>
        <Appear at={at + 26} style={{ fontSize: 15, color: C.faint, marginTop: 10 }}>
          Theme · Dark, the 9 colours as the file has them
        </Appear>
      </div>
      <Picture at={at + 8} src="showcase/talk/before.png" caption="Drawn by grafta from order.fig" />
    </div>
  );
}

function ReplyLight({ at }: { at: number }) {
  const frame = useCurrentFrame();
  return (
    <div style={{ display: "flex", gap: 56, alignItems: "flex-start" }}>
      <div style={{ flex: 1 }}>
        <Lead at={at}>{EXCHANGES[1]!.lead}</Lead>
        <Appear at={at + 14} style={{ fontSize: 21, fontWeight: 650, marginTop: 20 }}>
          Contrast: all 9 text layers pass WCAG 2.2 AA.
        </Appear>
        <div style={{ marginTop: 12, border: `1px solid ${C.line}`, borderRadius: 12, overflow: "hidden", maxWidth: 640, opacity: ramp(frame, at + 16, at + 24) }}>
          {CONTRAST.map((row, i) => {
            const p = ramp(frame, at + 20 + i * 5, at + 30 + i * 5);
            return (
              <div
                key={row.text}
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 120px 80px 50px",
                  alignItems: "center",
                  padding: "10px 16px",
                  fontSize: 17,
                  borderTop: i ? `1px solid ${C.line}` : "none",
                  opacity: p,
                }}
              >
                <span>{row.text}</span>
                <span style={{ display: "flex", gap: 6 }}>
                  {row.pair.map((c) => (
                    <span key={c} style={{ width: 22, height: 22, borderRadius: 6, background: c, border: `1px solid ${C.line}` }} />
                  ))}
                </span>
                <span style={{ fontFamily: MONO, fontWeight: 600 }}>{row.ratio}</span>
                <span style={{ color: C.deep, fontWeight: 800 }}>✓</span>
              </div>
            );
          })}
        </div>
      </div>
      <Picture at={at + 8} src="showcase/talk/light.png" caption="Drawn by grafta from order-light.fig" />
    </div>
  );
}

function ReplyCode({ at }: { at: number }) {
  const frame = useCurrentFrame();
  const lines = CODE.split("\n");
  const shown = Math.floor(interpolate(frame, [at + 10, at + 70], [0, lines.length], clamp));
  return (
    <div style={{ display: "flex", gap: 40, alignItems: "flex-start" }}>
      <div style={{ flex: 1 }}>
        <Lead at={at}>{EXCHANGES[2]!.lead}</Lead>
        <Appear
          at={at + 8}
          style={{
            marginTop: 18,
            background: "#16181D",
            color: "#E6E6E1",
            borderRadius: 14,
            padding: "16px 20px",
            fontFamily: MONO,
            fontSize: 15.5,
            lineHeight: 1.45,
            height: 380,
            overflow: "hidden",
            maxWidth: 640,
          }}
        >
          <div style={{ color: "#8A8E95", marginBottom: 6 }}>order.html</div>
          {lines.slice(0, shown).map((line, i) => (
            <div key={i} style={{ whiteSpace: "pre" }}>
              {line.length ? <CodeLine line={line} /> : "\u00a0"}
            </div>
          ))}
        </Appear>
      </div>
      <BrowserFrame at={at + 50} />
    </div>
  );
}

function CodeLine({ line }: { line: string }) {
  const parts = line.split(/(--[a-z-]+|#[0-9A-F]{6}|\d+px)/g);
  return (
    <>
      {parts.map((part, i) => (
        <span
          key={i}
          style={{
            color: part.startsWith("--") ? C.bud : part.startsWith("#") ? "#F0A47E" : /\d+px/.test(part) ? "#9CC8F5" : undefined,
          }}
        >
          {part}
        </span>
      ))}
    </>
  );
}

function BrowserFrame({ at }: { at: number }) {
  const pop = useSpring(at, 16);
  return (
    <div
      style={{
        width: 330,
        borderRadius: 14,
        overflow: "hidden",
        border: `1px solid ${C.line}`,
        boxShadow: "0 14px 40px rgb(15 17 21 / 0.14)",
        opacity: Math.min(1, pop * 1.5),
        transform: `translateY(${(1 - pop) * 24}px)`,
      }}
    >
      <div style={{ height: 34, background: C.sunk, display: "flex", alignItems: "center", gap: 7, padding: "0 12px" }}>
        {["#FF5F57", "#FEBC2E", "#28C840"].map((c) => (
          <div key={c} style={{ width: 10, height: 10, borderRadius: 5, background: c }} />
        ))}
        <div style={{ marginLeft: 10, fontSize: 13, color: C.muted, background: C.surface, borderRadius: 6, padding: "3px 10px" }}>
          order.html
        </div>
      </div>
      <Img src={staticFile("showcase/talk/code.png")} style={{ width: 330, display: "block" }} />
      <div style={{ fontSize: 13, color: C.faint, padding: "6px 12px", background: C.surface }}>The model's code, in a browser</div>
    </div>
  );
}

// ——— The result, open in Figma ———

const FIG = { x: 180, y: 70, w: 1560, h: 800 };

function FigmaWindow() {
  const frame = useCurrentFrame();
  const rise = useSpring(T.figma + 6, 17);
  const gone = ramp(frame, T.end, T.end + 12);
  if (frame < T.figma) return null;
  const selected = frame >= T.figma + 64;
  return (
    <div
      style={{
        position: "absolute",
        left: FIG.x,
        top: FIG.y,
        width: FIG.w,
        height: FIG.h,
        borderRadius: 14,
        overflow: "hidden",
        background: "#F5F5F5",
        boxShadow: "0 30px 80px rgb(15 17 21 / 0.28), 0 0 0 1px rgb(15 17 21 / 0.1)",
        opacity: Math.min(1, rise * 1.5) * (1 - gone),
        transform: `translateY(${(1 - rise) * 140}px) scale(${0.94 + rise * 0.06})`,
      }}
    >
      <TitleBar dark title="order-light.fig" />
      <div style={{ position: "absolute", top: 52, bottom: 0, left: 0, width: 300, background: "#fff", borderRight: "1px solid #E6E6E6" }}>
        <div style={{ padding: "16px 18px 10px", fontSize: 14, fontWeight: 650 }}>Layers</div>
        {LAYERS.map((layer, i) => {
          const p = ramp(frame, T.figma + 20 + i * 3, T.figma + 28 + i * 3);
          const isOrder = i === 0;
          return (
            <div
              key={layer.name}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 9,
                padding: `6px 18px 6px ${18 + layer.depth * 18}px`,
                fontSize: 14,
                opacity: p,
                background: isOrder && selected ? "#E5F4FF" : "transparent",
                fontWeight: isOrder ? 600 : 400,
              }}
            >
              <span style={{ fontFamily: MONO, fontSize: 12, color: "#8A8E95", width: 14 }}>
                {layer.type === "TEXT" ? "T" : layer.type === "VECTOR" ? "✎" : "#"}
              </span>
              <span style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{layer.name}</span>
            </div>
          );
        })}
      </div>
      <div style={{ position: "absolute", top: 52, bottom: 0, right: 0, width: 300, background: "#fff", borderLeft: "1px solid #E6E6E6" }}>
        <FigPanel selected={selected} />
      </div>
      {/* the canvas */}
      <div style={{ position: "absolute", top: 52, bottom: 0, left: 300, right: 300, display: "grid", placeItems: "center" }}>
        <div style={{ position: "relative" }}>
          <div style={{ position: "absolute", top: -24, left: 0, fontSize: 13, color: selected ? C.select : "#8A8E95" }}>Order</div>
          <Img src={staticFile("showcase/talk/light.png")} style={{ width: 420, display: "block" }} />
          {selected ? (
            <div style={{ position: "absolute", inset: -1, border: `2px solid ${C.select}` }}>
              {[
                [0, 0],
                [1, 0],
                [0, 1],
                [1, 1],
              ].map(([x, y]) => (
                <div
                  key={`${x}${y}`}
                  style={{
                    position: "absolute",
                    left: `calc(${x! * 100}% - 5px)`,
                    top: `calc(${y! * 100}% - 5px)`,
                    width: 8,
                    height: 8,
                    background: "#fff",
                    border: `1.5px solid ${C.select}`,
                  }}
                />
              ))}
              <div
                style={{
                  position: "absolute",
                  bottom: -30,
                  left: "50%",
                  transform: "translateX(-50%)",
                  background: C.select,
                  color: "#fff",
                  fontSize: 12,
                  padding: "3px 7px",
                  borderRadius: 4,
                }}
              >
                360 × 365
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function FigPanel({ selected }: { selected: boolean }) {
  const frame = useCurrentFrame();
  const p = ramp(frame, T.figma + 70, T.figma + 82);
  const row: CSSProperties = { display: "flex", justifyContent: "space-between", fontSize: 14, padding: "7px 0" };
  if (!selected) return <div style={{ padding: 18, fontSize: 14, color: "#8A8E95" }}>Design</div>;
  return (
    <div style={{ padding: 18, opacity: p }}>
      <div style={{ fontSize: 14, fontWeight: 650, marginBottom: 10 }}>Frame · Order</div>
      <div style={row}>
        <span style={{ color: "#8A8E95" }}>Auto layout</span>
        <span>Vertical, gap 20, padding 24</span>
      </div>
      <div style={{ height: 1, background: "#E6E6E6", margin: "12px 0" }} />
      <div style={{ fontSize: 14, fontWeight: 650, marginBottom: 6 }}>Variable modes</div>
      <div style={row}>
        <span style={{ color: "#8A8E95" }}>Theme</span>
        <span style={{ background: C.soft, color: C.deep, fontWeight: 650, padding: "2px 10px", borderRadius: 6 }}>Light</span>
      </div>
      <div style={{ height: 1, background: "#E6E6E6", margin: "12px 0" }} />
      <div style={{ fontSize: 14, fontWeight: 650, marginBottom: 6 }}>Fill</div>
      {[
        ["Background", "#F6F4EE"],
        ["Surface", "#FFFFFF"],
        ["Accent", "#13704B"],
      ].map(([name, c]) => (
        <div key={name} style={{ ...row, justifyContent: "flex-start", gap: 10 }}>
          <span style={{ width: 18, height: 18, borderRadius: 4, background: c, border: "1px solid #E6E6E6" }} />
          <span>{name}</span>
          <span style={{ marginLeft: "auto", fontFamily: MONO, fontSize: 12, color: "#8A8E95" }}>{c}</span>
        </div>
      ))}
    </div>
  );
}

// ——— Words under the picture ———

const CAPTIONS: { from: number; to: number; text: string }[] = [
  { from: T.dragFrom - 10, to: sendOf(0) + 10, text: "Get a Figma file, and add it to the chat." },
  { from: sendOf(0) + 10, to: sendOf(1), text: "Ask what's in it." },
  { from: sendOf(1), to: sendOf(2), text: "Change it, and check it. It saves a new file, never over yours." },
  { from: sendOf(2), to: T.figma, text: "Turn it into code, in whatever language you work in." },
  { from: T.figma + 10, to: T.end, text: "Open the result in Figma: real layers and variables, to keep editing." },
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
            top: 922,
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
      <div style={{ position: "absolute", bottom: 26, right: 40, fontSize: 15, color: C.faint, opacity: footnote }}>
        Requests and replies from a real session, shortened: {RECORDED}
      </div>
    </>
  );
}

// ——— The end ———

function EndCard() {
  const frame = useCurrentFrame();
  if (frame < T.end + 6) return null;
  const at = T.end + 8;
  return (
    <AbsoluteFill style={{ display: "grid", placeItems: "center", textAlign: "center" }}>
      <div>
        <Appear at={at} style={{ display: "flex", justifyContent: "center" }}>
          <Img src={staticFile("brand/logo-light.svg")} style={{ height: 76 }} />
        </Appear>
        <Appear at={at + 6} style={{ fontSize: 64, fontWeight: 650, letterSpacing: "-0.035em", marginTop: 36 }}>
          Talk to your Figma files.
        </Appear>
        <Appear at={at + 12} style={{ fontSize: 28, color: C.muted, marginTop: 16 }}>
          No Figma account, no Figma app running, no API limits.
        </Appear>
        <Appear at={at + 18} style={{ fontSize: 20, color: C.faint, marginTop: 40 }}>
          {APPS.map((a) => a.name).join("  ·  ")}
        </Appear>
        <Appear
          at={at + 24}
          style={{
            display: "inline-block",
            marginTop: 40,
            fontSize: 24,
            fontFamily: MONO,
            background: C.ink,
            color: "#fff",
            padding: "14px 26px",
            borderRadius: 12,
          }}
        >
          github.com/n0ss4/grafta
        </Appear>
        <Appear at={at + 30} style={{ fontSize: 16, color: C.faint, marginTop: 28 }}>
          Free and open source. Not affiliated with Figma.
        </Appear>
      </div>
    </AbsoluteFill>
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
  [T.dragTo, INPUT.x + 46, INPUT.y - 4],
  [T.dragTo + 30, INPUT.x + 900, INPUT.y + 120],
  [T.figma + 30, FIG.x + 900, FIG.y + 560],
  [T.figma + 60, FIG.x + 760, FIG.y + 300],
  [T.end, FIG.x + 1100, FIG.y + 600],
];
const CLICKS = [T.click, T.dragFrom, T.figma + 62];

function Cursor({ frame }: { frame: number }) {
  if (frame >= T.end) return null;
  const xs = PATH.map((p) => p[1]);
  const ys = PATH.map((p) => p[2]);
  const fs = PATH.map((p) => p[0]);
  const x = interpolate(frame, fs, xs, { ...clamp, easing: Easing.inOut(Easing.cubic) });
  const y = interpolate(frame, fs, ys, { ...clamp, easing: Easing.inOut(Easing.cubic) });
  const click = CLICKS.find((c) => frame >= c && frame < c + 14);
  const r = click !== undefined ? ramp(frame, click, click + 14) : 0;
  const hidden = frame > T.dragTo + 30 && frame < T.figma + 6;
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
        style={{ position: "absolute", left: x - 4, top: y - 2, opacity: hidden ? 0 : 1, filter: "drop-shadow(0 2px 3px rgb(0 0 0 / 0.3))" }}
      >
        <path d="M4 2 L4 19 L8.5 15 L11.5 22 L14.5 20.7 L11.6 14 L18 14 Z" fill="#111" stroke="#fff" strokeWidth={1.5} strokeLinejoin="round" />
      </svg>
    </>
  );
}
