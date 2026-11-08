import type { CSSProperties, ReactNode } from "react";
import { Img, interpolate, staticFile, useCurrentFrame } from "remotion";
import { DOCK, FILE, FigFile, dockX } from "./Dock";
import { CONTRAST, EXCHANGES } from "./talk";
import {
  Appear,
  C,
  MONO,
  STEP_GAP,
  T,
  TrafficLights,
  cardAt,
  clamp,
  openCodeAt,
  ramp,
  replyAt,
  sendOf,
  stepAt,
  useSpring,
} from "./shared";

// Claude Desktop's window, in its colours and typefaces.
const K = {
  page: "#FAF9F5",
  side: "#F5F4ED",
  ink: "#141413",
  muted: "#73726C",
  line: "#E8E6DC",
  bubble: "#F0EEE6",
  orange: "#C6613F",
  spark: "#D97757",
};
const SANS = "'Anthropic Sans', 'Inter Variable', system-ui, sans-serif";
const SERIF = "'Anthropic Serif', Georgia, serif";

export const WIN = { x: 190, y: 54, w: 1540, h: 846 };
const SIDE = 268;
const COL = { left: SIDE + 120, width: 900 };
export const INPUT = { x: WIN.x + COL.left + 60, y: WIN.y + WIN.h - 92 };
export const CARD = { x: 760, y: 368 };
/** Where the file is dropped: the box under the greeting. */
const DROP_Y = WIN.y + 400;

/** Claude's mark, a spark of rays. */
export function Spark({ size, color = K.spark, spin = 0 }: { size: number; color?: string; spin?: number }) {
  const rays = [0, 32, 61, 95, 124, 152, 185, 213, 247, 274, 303, 333];
  return (
    <svg width={size} height={size} viewBox="-12 -12 24 24" style={{ transform: `rotate(${spin}deg)` }}>
      {rays.map((a, i) => (
        <line
          key={a}
          x1={0}
          y1={0}
          x2={Math.cos((a * Math.PI) / 180) * (i % 2 ? 9.5 : 11)}
          y2={Math.sin((a * Math.PI) / 180) * (i % 2 ? 9.5 : 11)}
          stroke={color}
          strokeWidth={2.4}
          strokeLinecap="round"
        />
      ))}
    </svg>
  );
}

export function ClaudeWindow() {
  const frame = useCurrentFrame();
  const open = useSpring(T.open, 16);
  const back = ramp(frame, T.vscode, T.vscode + 18);
  if (frame < T.open || frame > T.vscode + 40) return null;
  const fromX = dockX(0) - (WIN.x + WIN.w / 2);
  const fromY = DOCK.y + DOCK.tile / 2 - (WIN.y + WIN.h / 2);
  const started = frame >= sendOf(0);
  return (
    <div
      style={{
        position: "absolute",
        left: WIN.x,
        top: WIN.y,
        width: WIN.w,
        height: WIN.h,
        borderRadius: 14,
        background: K.page,
        overflow: "hidden",
        fontFamily: SANS,
        color: K.ink,
        boxShadow: "0 30px 80px rgb(15 17 21 / 0.22), 0 0 0 1px rgb(15 17 21 / 0.1)",
        opacity: Math.min(1, open * 1.6),
        transform: `translate(${(1 - open) * fromX - back * 120}px, ${(1 - open) * fromY - back * 20}px) scale(${
          (0.12 + 0.88 * open) * (1 - back * 0.1)
        })`,
        filter: `brightness(${1 - back * 0.08})`,
      }}
    >
      <Sidebar started={started} />
      <div style={{ position: "absolute", left: SIDE, top: 0, right: 0, height: 52, display: "flex", alignItems: "center", padding: "0 22px" }}>
        <div style={{ fontSize: 15, fontWeight: 500, opacity: started ? ramp(frame, sendOf(0) + 30, sendOf(0) + 40) : 0 }}>
          Order card in a Figma file <span style={{ color: K.muted }}>⌄</span>
        </div>
        <div
          style={{
            marginLeft: "auto",
            fontSize: 14,
            padding: "6px 14px",
            border: `1px solid ${K.line}`,
            borderRadius: 8,
            background: "#fff",
          }}
        >
          Share
        </div>
      </div>
      {!started ? <Greeting /> : null}
      <div style={{ position: "absolute", top: 60, bottom: 150, left: SIDE, right: 0, overflow: "hidden" }}>
        {EXCHANGES.map((_, k) => (
          <ExchangeView key={k} k={k} />
        ))}
      </div>
      <InputBox />
    </div>
  );
}

function Sidebar({ started }: { started: boolean }) {
  const frame = useCurrentFrame();
  const item = (label: string, icon: ReactNode, active = false): ReactNode => (
    <div
      key={label}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "8px 10px",
        borderRadius: 8,
        fontSize: 15,
        background: active ? "#EAE8DF" : "transparent",
      }}
    >
      <span style={{ width: 18, color: K.muted, display: "grid", placeItems: "center" }}>{icon}</span>
      {label}
    </div>
  );
  const recents = ["Tokens for the marketing site", "Contrast audit, checkout", "Onboarding deck notes"];
  return (
    <div style={{ position: "absolute", left: 0, top: 0, bottom: 0, width: SIDE, background: K.side, borderRight: `1px solid ${K.line}`, padding: "18px 14px" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
        <TrafficLights />
      </div>
      <div style={{ fontFamily: SERIF, fontSize: 25, fontWeight: 500, margin: "22px 10px 18px", letterSpacing: "-0.01em" }}>Claude</div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "8px 10px", fontSize: 15, fontWeight: 500 }}>
        <span style={{ width: 22, height: 22, borderRadius: 11, background: K.orange, color: "#fff", display: "grid", placeItems: "center", fontSize: 16 }}>+</span>
        New chat
      </div>
      {item("Chats", <Glyph d="M4 5h12v8H8l-4 3z" />)}
      {item("Projects", <Glyph d="M3 6h5l2 2h7v8H3z" />)}
      {item("Artifacts", <Glyph d="M5 4h10v12H5zM8 8h4M8 11h4" />)}
      {item("Code", <Glyph d="M7 6l-4 4 4 4M13 6l4 4-4 4" />)}
      <div style={{ fontSize: 13, color: K.muted, margin: "22px 10px 6px" }}>Recents</div>
      {started ? (
        <div style={{ opacity: ramp(frame, sendOf(0) + 30, sendOf(0) + 40) }}>{item("Order card in a Figma file", null, true)}</div>
      ) : null}
      {recents.map((r) => (
        <div key={r} style={{ padding: "8px 10px 8px 40px", fontSize: 15, color: K.ink, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
          {r}
        </div>
      ))}
      <div style={{ position: "absolute", left: 14, right: 14, bottom: 16, display: "flex", alignItems: "center", gap: 10, padding: "8px 10px" }}>
        <div style={{ width: 30, height: 30, borderRadius: 15, background: "#3D3D3A", color: "#fff", display: "grid", placeItems: "center", fontSize: 13, fontWeight: 600 }}>
          N
        </div>
        <div>
          <div style={{ fontSize: 14, fontWeight: 500 }}>Nossair</div>
          <div style={{ fontSize: 12, color: K.muted }}>Max plan</div>
        </div>
      </div>
    </div>
  );
}

function Glyph({ d }: { d: string }) {
  return (
    <svg width={18} height={18} viewBox="0 0 20 20">
      <path d={d} fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

function Greeting() {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: "absolute",
        left: SIDE,
        right: 0,
        top: 270,
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        gap: 16,
        opacity: ramp(frame, T.open + 12, T.open + 24),
      }}
    >
      <Spark size={40} />
      <div style={{ fontFamily: SERIF, fontSize: 44, fontWeight: 400, letterSpacing: "-0.015em" }}>Good afternoon, Nossair</div>
    </div>
  );
}

// ——— The file, carried from the desktop into the box ———

export function DesktopFile() {
  const frame = useCurrentFrame();
  const shown = ramp(frame, T.open + 10, T.open + 22);
  const t = ramp(frame, T.dragFrom, T.dragTo);
  if (frame < T.open || frame > T.dragTo + 12) return null;
  const x = interpolate(t, [0, 1], [FILE.x, INPUT.x + 30]);
  const y = interpolate(t, [0, 1], [FILE.y, INPUT.y - 40]) - Math.sin(Math.PI * t) * 80;
  return (
    <>
      <div style={{ position: "absolute", left: FILE.x - 48, top: FILE.y - 44, opacity: shown * (t > 0 ? 0.35 : 1) * (1 - ramp(frame, T.dragTo, T.dragTo + 10)) }}>
        <FigFile />
      </div>
    </>
  );
}

/** The file while it is carried, drawn above the windows. */
export function carriedAt(frame: number) {
  const t = ramp(frame, T.dragFrom, T.dragTo);
  const x = interpolate(t, [0, 1], [FILE.x, INPUT.x + 30]);
  const y = interpolate(t, [0, 1], [FILE.y, DROP_Y]) - Math.sin(Math.PI * t) * 80;
  return { t, x, y };
}

export function CarriedFile() {
  const frame = useCurrentFrame();
  const { t, x, y } = carriedAt(frame);
  return (
    <>
      {frame < T.dragTo && t > 0 ? (
        <div style={{ position: "absolute", left: x - 48, top: y - 44, transform: `scale(${1 - t * 0.3}) rotate(${Math.sin(Math.PI * t) * -6}deg)`, opacity: 0.95 }}>
          <FigFile />
        </div>
      ) : null}
    </>
  );
}

// ——— The box you type in ———

function Attachment({ pop = 1 }: { pop?: number }) {
  return (
    <div
      style={{
        width: 128,
        height: 104,
        borderRadius: 10,
        border: `1px solid ${K.line}`,
        background: "#fff",
        padding: "10px 12px",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        transform: `scale(${pop})`,
        transformOrigin: "left bottom",
        boxShadow: "0 1px 3px rgb(0 0 0 / 0.05)",
      }}
    >
      <div style={{ fontSize: 13, fontWeight: 500, lineHeight: 1.3 }}>order.fig</div>
      <div style={{ fontSize: 11, color: K.muted }}>28 KB</div>
      <div
        style={{
          alignSelf: "flex-start",
          fontSize: 10,
          fontWeight: 600,
          border: `1px solid ${K.line}`,
          borderRadius: 4,
          padding: "1px 5px",
          color: K.muted,
        }}
      >
        FIG
      </div>
    </div>
  );
}

function InputBox() {
  const frame = useCurrentFrame();
  const attached = frame >= T.dragTo && frame < sendOf(0);
  const attachPop = useSpring(T.dragTo, 13);
  let text = "";
  for (let k = 0; k < EXCHANGES.length; k++) {
    const start = T.types[k] ?? 0;
    const words = EXCHANGES[k]!.words;
    if (frame >= start && frame < sendOf(k)) text = words.slice(0, Math.max(0, Math.floor(frame - start)));
  }
  const caret = Math.floor(frame / 15) % 2 === 0;
  const hover = frame >= T.dragFrom + 20 && frame < T.dragTo;
  const started = frame >= sendOf(0);
  // before the first message the box sits under the greeting, then at the bottom
  const settle = ramp(frame, sendOf(0) - 4, sendOf(0) + 8);
  const top = interpolate(settle, [0, 1], [350, WIN.h - (attached ? 250 : 150)]);
  return (
    <div style={{ position: "absolute", left: COL.left + 30, width: COL.width - 60, top: attached ? top - 104 : top }}>
      <div
        style={{
          borderRadius: 20,
          border: `1px solid ${hover ? K.spark : K.line}`,
          background: "#fff",
          boxShadow: hover ? `0 0 0 3px rgb(217 119 87 / 0.18)` : "0 4px 20px rgb(0 0 0 / 0.05)",
          padding: "14px 16px 12px 18px",
        }}
      >
        {attached ? (
          <div style={{ marginBottom: 12 }}>
            <Attachment pop={attachPop} />
          </div>
        ) : null}
        <div style={{ fontSize: 19, color: text ? K.ink : K.muted, minHeight: 28 }}>
          {text || (hover ? "Drop files here to add them to the chat" : started ? "Reply to Claude…" : "How can I help you today?")}
          {text && caret ? <span style={{ color: K.ink }}>|</span> : null}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 10 }}>
          {["+", "⚙"].map((g) => (
            <div key={g} style={{ width: 32, height: 32, borderRadius: 8, border: `1px solid ${K.line}`, display: "grid", placeItems: "center", color: K.muted, fontSize: 17 }}>
              {g}
            </div>
          ))}
          <div style={{ marginLeft: "auto", fontSize: 14, color: K.muted }}>Opus 5.5 ⌄</div>
          <div
            style={{
              width: 34,
              height: 34,
              borderRadius: 9,
              background: K.orange,
              opacity: text ? 1 : 0.45,
              color: "#fff",
              display: "grid",
              placeItems: "center",
              fontSize: 18,
              fontWeight: 600,
            }}
          >
            ↑
          </div>
        </div>
      </div>
    </div>
  );
}

// ——— One request, Claude's calls to grafta, and the reply ———

const SCROLL: Record<number, [number, number][]> = {
  0: [
    [replyAt(0) + 22, 0],
    [replyAt(0) + 40, 110],
  ],
  1: [
    [replyAt(1) + 34, 0],
    [replyAt(1) + 56, 236],
  ],
};

function ExchangeView({ k }: { k: number }) {
  const frame = useCurrentFrame();
  const send = sendOf(k);
  const next = k + 1 < EXCHANGES.length ? sendOf(k + 1) : T.total + 100;
  if (frame < send || frame > next + 10) return null;
  const leave = ramp(frame, next, next + 9);
  const keys = SCROLL[k];
  const scroll = keys ? interpolate(frame, keys.map((p) => p[0]), keys.map((p) => p[1]), clamp) : 0;
  const ex = EXCHANGES[k]!;
  return (
    <div
      style={{
        position: "absolute",
        left: COL.left - SIDE,
        width: COL.width,
        top: 20,
        opacity: 1 - leave,
        transform: `translateY(${-leave * 160 - scroll}px)`,
      }}
    >
      <Appear at={send} style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 8 }}>
        {k === 0 ? <Attachment /> : null}
        <div style={{ background: K.bubble, borderRadius: 16, padding: "12px 18px", fontSize: 19 }}>{ex.words}</div>
      </Appear>
      <div style={{ marginTop: 18, display: "flex", flexDirection: "column", gap: 6 }}>
        {ex.steps.map((step, i) => (
          <ToolRow key={i} at={stepAt(k, i)} tools={step.tools} picture={k === 0 && i === 1 ? "showcase/talk/before.png" : k === 1 && i === 1 ? "showcase/talk/light.png" : undefined} />
        ))}
      </div>
      <div style={{ marginTop: 16 }}>
        {frame >= send + 6 ? <Thinking until={replyAt(k)} /> : null}
        {k === 0 ? <ReplyInside /> : k === 1 ? <ReplyLight /> : <ReplyCode />}
      </div>
    </div>
  );
}

/** A collapsed call, as Claude shows a connector's tool, opened when it drew a picture. */
function ToolRow({ at, tools, picture }: { at: number; tools: string[]; picture?: string | undefined }) {
  const frame = useCurrentFrame();
  const done = frame >= at + STEP_GAP - 4;
  const unique = [...new Set(tools)];
  return (
    <Appear at={at} dy={6}>
      <div
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 10,
          fontSize: 16,
          color: K.muted,
          border: `1px solid ${K.line}`,
          borderRadius: 10,
          padding: "7px 12px",
          background: "#fff",
        }}
      >
        <MarkSmall />
        <span style={{ color: K.ink, fontWeight: 500 }}>grafta</span>
        <span style={{ fontFamily: MONO, fontSize: 14.5 }}>{unique.join(", ")}</span>
        <span style={{ marginLeft: 6, width: 14, display: "grid", placeItems: "center" }}>
          {done ? (
            <span style={{ fontSize: 12 }}>{picture ? "⌄" : "›"}</span>
          ) : (
            <span style={{ width: 12, height: 12, borderRadius: 6, border: `2px solid ${K.line}`, borderTopColor: K.spark, transform: `rotate(${(frame - at) * 24}deg)` }} />
          )}
        </span>
      </div>
      {picture && done ? (
        <Appear at={at + STEP_GAP - 2} dy={8} style={{ marginTop: 8 }}>
          <Img src={staticFile(picture)} style={{ width: 230, borderRadius: 10, border: `1px solid ${K.line}`, display: "block" }} />
        </Appear>
      ) : null}
    </Appear>
  );
}

function MarkSmall() {
  return (
    <svg width={16} height={16} viewBox="0 0 64 64">
      <path d="M14 46 L32 18 L50 46" fill="none" stroke={K.ink} strokeWidth={7} strokeLinecap="round" strokeLinejoin="round" />
      <circle cx={14} cy={46} r={9} fill={K.ink} />
      <circle cx={32} cy={18} r={9} fill={K.ink} />
      <circle cx={50} cy={46} r={9} fill={C.sap} />
    </svg>
  );
}

const prose: CSSProperties = { fontFamily: SERIF, fontSize: 21, lineHeight: 1.55, color: K.ink };

/** Text that streams in, word by word. */
function Stream({ at, text, style, perFrame = 2.4 }: { at: number; text: string; style?: CSSProperties; perFrame?: number }) {
  const frame = useCurrentFrame();
  const words = text.split(" ");
  const shown = Math.max(0, Math.floor((frame - at) * perFrame));
  if (frame < at) return null;
  return <div style={{ ...prose, ...style }}>{words.slice(0, shown).join(" ")}</div>;
}

function Thinking({ until }: { until: number }) {
  const frame = useCurrentFrame();
  if (frame >= until) return null;
  return <Spark size={22} spin={frame * 6} />;
}

function ReplyInside() {
  const at = replyAt(0);
  const note = EXCHANGES[0]!.note;
  return (
    <div>
      <Stream at={at} text={EXCHANGES[0]!.lead} />
      {note ? <Stream at={at + 26} text={note} style={{ marginTop: 12, color: K.muted }} /> : null}
    </div>
  );
}

function ReplyLight() {
  const frame = useCurrentFrame();
  const at = replyAt(1);
  return (
    <div>
      <Stream at={at} text={EXCHANGES[1]!.lead} />
      <Appear at={at + 18} style={{ ...prose, fontWeight: 650, marginTop: 14 }}>
        Contrast: all 9 text layers pass WCAG 2.2 AA in Light mode.
      </Appear>
      <div style={{ marginTop: 12, fontFamily: SANS, fontSize: 16.5, opacity: ramp(frame, at + 22, at + 30) }}>
        <div style={{ display: "grid", gridTemplateColumns: "1.3fr 1.5fr 70px 60px", fontWeight: 600, padding: "8px 4px", borderBottom: `1px solid ${K.line}` }}>
          <span>Text</span>
          <span>Colour on background</span>
          <span>Ratio</span>
          <span>Needs</span>
        </div>
        {CONTRAST.map((row, i) => (
          <div
            key={row.text}
            style={{
              display: "grid",
              gridTemplateColumns: "1.3fr 1.5fr 70px 60px",
              padding: "8px 4px",
              borderBottom: `1px solid ${K.line}`,
              opacity: ramp(frame, at + 26 + i * 4, at + 34 + i * 4),
            }}
          >
            <span>{row.text}</span>
            <span style={{ fontFamily: MONO, fontSize: 13.5 }}>
              {row.pair[0]} on {row.pair[1]}
            </span>
            <span style={{ fontWeight: row.ratio === "4.58" ? 700 : 400 }}>{row.ratio}</span>
            <span>{row.needs}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function ReplyCode() {
  const frame = useCurrentFrame();
  const at = replyAt(2);
  const pressed = frame >= openCodeAt() && frame < openCodeAt() + 8;
  return (
    <div>
      <Stream at={at} text={EXCHANGES[2]!.lead} />
      <Appear at={cardAt()} style={{ marginTop: 16 }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 14,
            width: 420,
            border: `1px solid ${K.line}`,
            borderRadius: 12,
            padding: "12px 14px",
            background: pressed ? "#F0EEE6" : "#fff",
            fontFamily: SANS,
          }}
        >
          <div style={{ width: 40, height: 40, borderRadius: 8, background: K.bubble, display: "grid", placeItems: "center", fontFamily: MONO, fontSize: 13, color: K.muted }}>
            {"</>"}
          </div>
          <div>
            <div style={{ fontSize: 15, fontWeight: 500 }}>Order</div>
            <div style={{ fontSize: 13, color: K.muted }}>Code · HTML</div>
          </div>
          <div style={{ marginLeft: "auto", fontSize: 13, color: K.muted, border: `1px solid ${K.line}`, borderRadius: 7, padding: "5px 10px" }}>
            order.html in Downloads
          </div>
        </div>
      </Appear>
    </div>
  );
}
