import type { ReactNode } from "react";
import { Img, interpolate, staticFile, useCurrentFrame } from "remotion";
import { ORDER_HTML } from "./talk";
import { Appear, T, TrafficLights, clamp, openFigAt, ramp, useSpring } from "./shared";

// VS Code, in its Dark Modern theme, with the model's order.html open beside
// the page it draws.
const V = {
  title: "#181818",
  side: "#181818",
  editor: "#1F1F1F",
  border: "#2B2B2B",
  text: "#CCCCCC",
  dim: "#6E7681",
  tabLine: "#0078D4",
  select: "#37373D",
  // Dark+ tokens
  tag: "#569CD6",
  attr: "#9CDCFE",
  str: "#CE9178",
  sel: "#D7BA7D",
  num: "#B5CEA8",
  comment: "#6A9955",
  punct: "#808080",
  green: "#73C991",
};
const UI = "'Inter Variable', Inter, system-ui, sans-serif";
const CODE = "Menlo, 'SF Mono', ui-monospace, monospace";

export const VS = { x: 150, y: 54, w: 1620, h: 846 };
/** Where order-light.fig sits in the explorer. */
export const FIG_ROW = { x: VS.x + 150, y: VS.y + 108 };
const LINES = ORDER_HTML.split("\n").slice(0, 34);

export function VSCodeWindow() {
  const frame = useCurrentFrame();
  const rise = useSpring(T.vscode, 17);
  const back = ramp(frame, T.figma, T.figma + 18);
  if (frame < T.vscode || frame > T.figma + 40) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: VS.x,
        top: VS.y,
        width: VS.w,
        height: VS.h,
        borderRadius: 12,
        overflow: "hidden",
        background: V.editor,
        fontFamily: UI,
        color: V.text,
        boxShadow: "0 30px 80px rgb(15 17 21 / 0.3), 0 0 0 1px rgb(0 0 0 / 0.4)",
        opacity: Math.min(1, rise * 1.5),
        transform: `translate(${-back * 100}px, ${(1 - rise) * 140 - back * 16}px) scale(${(0.94 + rise * 0.06) * (1 - back * 0.08)})`,
        filter: `brightness(${1 - back * 0.2})`,
      }}
    >
      {/* title bar */}
      <div style={{ height: 38, background: V.title, borderBottom: `1px solid ${V.border}`, display: "flex", alignItems: "center", padding: "0 14px", position: "relative" }}>
        <TrafficLights size={12} />
        <div style={{ position: "absolute", left: "50%", transform: "translateX(-50%)", fontSize: 13, background: "#2A2A2A", border: `1px solid ${V.border}`, borderRadius: 6, padding: "4px 120px", color: V.text }}>
          Downloads
        </div>
      </div>
      <div style={{ position: "absolute", top: 38, bottom: 24, left: 0, right: 0, display: "flex" }}>
        <ActivityBar />
        <Explorer />
        <div style={{ flex: 1, display: "flex", minWidth: 0 }}>
          <Editor />
          <Preview />
        </div>
      </div>
      {/* status bar */}
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 24, background: V.title, borderTop: `1px solid ${V.border}`, display: "flex", alignItems: "center", gap: 18, padding: "0 12px", fontSize: 12, color: V.text }}>
        <span>⎇ main</span>
        <span style={{ marginLeft: "auto" }}>Ln 12, Col 5</span>
        <span>Spaces: 2</span>
        <span>UTF-8</span>
        <span>HTML</span>
      </div>
    </div>
  );
}

function ActivityBar() {
  const icons = [
    "M6 3h7l4 4v11H6z M13 3v4h4",
    "M9 4a5 5 0 1 1 0 10a5 5 0 0 1 0-10z M13 13l5 5",
    "M7 4v12 M7 8a4 4 0 0 0 8 0 M15 4v4",
    "M4 4h5v5H4z M11 4h5v5h-5z M4 11h5v5H4z M11 11h5v5h-5z",
  ];
  return (
    <div style={{ width: 48, background: V.side, borderRight: `1px solid ${V.border}`, display: "flex", flexDirection: "column", alignItems: "center", gap: 22, paddingTop: 14 }}>
      {icons.map((d, i) => (
        <div key={d} style={{ position: "relative", width: 48, display: "grid", placeItems: "center" }}>
          {i === 0 ? <div style={{ position: "absolute", left: 0, top: -6, bottom: -6, width: 2, background: V.tabLine }} /> : null}
          <svg width={24} height={24} viewBox="0 0 22 22">
            <path d={d} fill="none" stroke={i === 0 ? "#D7D7D7" : "#868686"} strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </div>
      ))}
    </div>
  );
}

function Explorer() {
  const frame = useCurrentFrame();
  const row = (name: string, icon: ReactNode, extra?: ReactNode, active = false) => (
    <div
      key={name}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "3px 10px 3px 30px",
        fontSize: 13.5,
        background: active ? V.select : "transparent",
        outline: active ? "1px solid #0078D4" : "none",
        outlineOffset: -1,
      }}
    >
      {icon}
      <span style={{ color: active ? "#fff" : V.text }}>{name}</span>
      <span style={{ marginLeft: "auto" }}>{extra}</span>
    </div>
  );
  const fig = <Img src={staticFile("logos/figma.png")} style={{ width: 16, margin: -1 }} />;
  const html = <span style={{ color: "#E37933", fontFamily: CODE, fontSize: 12, fontWeight: 700, width: 14 }}>{"<>"}</span>;
  return (
    <div style={{ width: 250, background: V.side, borderRight: `1px solid ${V.border}` }}>
      <div style={{ fontSize: 11, letterSpacing: "0.06em", padding: "12px 20px", color: V.text }}>EXPLORER</div>
      <div style={{ fontSize: 11.5, fontWeight: 700, padding: "4px 12px", letterSpacing: "0.03em" }}>⌄ DOWNLOADS</div>
      {row("order-light.fig", fig, undefined, frame >= openFigAt())}
      {row("order.fig", fig)}
      <div style={{ opacity: ramp(frame, T.vscode + 6, T.vscode + 14) }}>
        {row("order.html", html, <span style={{ color: V.green, fontSize: 12, fontWeight: 600 }}>U</span>, frame < openFigAt())}
      </div>
    </div>
  );
}

function Editor() {
  const frame = useCurrentFrame();
  const shown = Math.floor(interpolate(frame, [T.vscode + 12, T.vscode + 70], [0, LINES.length], clamp));
  // the theme's tokens, picked out once the file is shown
  const mark = ramp(frame, T.vscode + 96, T.vscode + 106) * (1 - ramp(frame, T.figma - 10, T.figma));
  return (
    <div style={{ flex: 1.15, minWidth: 0, borderRight: `1px solid ${V.border}` }}>
      <Tabs names={["order.html"]} />
      <div style={{ fontSize: 12.5, color: V.dim, padding: "4px 16px", borderBottom: `1px solid ${V.border}` }}>Downloads › order.html</div>
      <div style={{ fontFamily: CODE, fontSize: 14.5, lineHeight: "21.5px", paddingTop: 6, position: "relative" }}>
        {/* the :root and dark tokens, lines 11 to 33 */}
        <div style={{ position: "absolute", left: 0, right: 0, top: 6 + 10 * 21.5, height: 23 * 21.5, background: `rgb(38 79 120 / ${0.42 * mark})` }} />
        {LINES.slice(0, shown).map((line, i) => (
          <div key={i} style={{ display: "flex", whiteSpace: "pre", position: "relative" }}>
            <span style={{ width: 56, textAlign: "right", paddingRight: 20, color: V.dim, flex: "none" }}>{i + 1}</span>
            <Highlight line={line} css={i >= LINES.findIndex((l) => l.includes("<style>")) + 1} />
          </div>
        ))}
      </div>
    </div>
  );
}

function Tabs({ names, icon }: { names: string[]; icon?: ReactNode }) {
  return (
    <div style={{ height: 36, background: V.side, display: "flex", borderBottom: `1px solid ${V.border}` }}>
      {names.map((name) => (
        <div key={name} style={{ display: "flex", alignItems: "center", gap: 8, padding: "0 16px", background: V.editor, borderTop: `1px solid ${V.tabLine}`, fontSize: 13.5, color: "#fff" }}>
          {icon ?? <span style={{ color: "#E37933", fontFamily: CODE, fontSize: 12, fontWeight: 700 }}>{"<>"}</span>}
          {name}
          <span style={{ color: V.dim, marginLeft: 8 }}>×</span>
        </div>
      ))}
    </div>
  );
}

/** Dark+ colours for the lines of an HTML file with a style block. */
function Highlight({ line, css }: { line: string; css: boolean }) {
  if (css && !line.includes("</style>")) {
    const comment = /^(\s*)(\/\*.*\*\/)\s*$/.exec(line);
    if (comment) return <span style={{ color: V.comment }}>{line}</span>;
    const decl = /^(\s*)(--[\w-]+|[\w-]+)(\s*:\s*)(.*?)(;?)$/.exec(line);
    if (decl && !line.trim().endsWith("{")) {
      const [, ws, prop, colon, value, semi] = decl;
      return (
        <span>
          {ws}
          <span style={{ color: V.attr }}>{prop}</span>
          <span style={{ color: V.text }}>{colon}</span>
          <Value value={value ?? ""} />
          <span style={{ color: V.text }}>{semi}</span>
        </span>
      );
    }
    if (line.trim().endsWith("{")) {
      const at = line.lastIndexOf("{");
      return (
        <span>
          <span style={{ color: V.sel }}>{line.slice(0, at)}</span>
          <span style={{ color: V.text }}>{"{"}</span>
        </span>
      );
    }
    return <span style={{ color: V.text }}>{line}</span>;
  }
  // HTML: tags, attributes and strings
  const parts = line.split(/(<\/?[\w!-]+|\/?>|[\w-]+=|"[^"]*")/g);
  return (
    <span>
      {parts.map((part, i) => {
        let color = V.text;
        if (/^<\/?[\w!-]+$/.test(part)) color = V.tag;
        else if (/^\/?>$/.test(part)) color = V.punct;
        else if (/^[\w-]+=$/.test(part)) color = V.attr;
        else if (/^"[^"]*"$/.test(part)) color = V.str;
        return (
          <span key={i} style={{ color }}>
            {part}
          </span>
        );
      })}
    </span>
  );
}

function Value({ value }: { value: string }) {
  const parts = value.split(/(#[0-9A-Fa-f]{3,8}|\d+(?:\.\d+)?(?:px|%)?|"[^"]*")/g);
  return (
    <>
      {parts.map((part, i) => {
        const swatch = /^#[0-9A-Fa-f]{6}$/.test(part);
        return (
          <span key={i} style={{ color: /^"/.test(part) ? V.str : /^#|^\d/.test(part) ? V.num : V.str }}>
            {swatch ? (
              <span style={{ display: "inline-block", width: 11, height: 11, border: "1px solid #888", background: part, marginRight: 4, verticalAlign: -1 }} />
            ) : null}
            {part}
          </span>
        );
      })}
    </>
  );
}

function Preview() {
  const frame = useCurrentFrame();
  const open = ramp(frame, T.vscode + 64, T.vscode + 80);
  return (
    <div style={{ flex: 0.85, minWidth: 0, opacity: open, background: V.editor, position: "relative" }}>
      <Tabs names={["Preview order.html"]} icon={<span style={{ color: "#3794FF" }}>◫</span>} />
      <div style={{ height: 34, display: "flex", alignItems: "center", gap: 10, padding: "0 12px", borderBottom: `1px solid ${V.border}`, fontSize: 12.5, color: V.dim }}>
        <span>‹ ›  ↻</span>
        <span style={{ flex: 1, background: "#2A2A2A", borderRadius: 4, padding: "3px 10px", color: V.text }}>file:///Downloads/order.html</span>
      </div>
      <div style={{ position: "absolute", top: 70, bottom: 0, left: 0, right: 0, background: "#F6F4EE", display: "flex", justifyContent: "center", paddingTop: 40 }}>
        <Appear at={T.vscode + 72} dy={10}>
          <Img src={staticFile("showcase/talk/code.png")} style={{ width: 430, display: "block" }} />
        </Appear>
      </div>
    </div>
  );
}
