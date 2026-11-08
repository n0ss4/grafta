import "@fontsource-variable/inter";
import { loadFont } from "@remotion/fonts";
import type { CSSProperties, ReactNode } from "react";
import { Easing, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";

export const WIDTH = 1920;
export const HEIGHT = 1080;

// Claude Desktop's own typefaces, when prepare.mjs found them; Inter otherwise.
for (const [family, file] of [
  ["Anthropic Sans", "fonts/sans.ttf"],
  ["Anthropic Serif", "fonts/serif.ttf"],
] as const) {
  loadFont({ family, url: staticFile(file), weight: "300 800" }).catch(() => undefined);
}

export const C = {
  bg: "#E8E8E4",
  ink: "#0F1115",
  muted: "#4A4E55",
  faint: "#8A8E95",
  line: "#D9DAD5",
  sap: "#1F9D6B",
  deep: "#13704B",
  bud: "#C6F26B",
  soft: "#E3F4EA",
  select: "#0D99FF",
};
export const SANS = "'Inter Variable', Inter, system-ui, sans-serif";
export const MONO = "ui-monospace, 'SF Mono', Menlo, monospace";

export * from "./timeline";

export const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;
const ease = Easing.bezier(0.25, 0.1, 0.25, 1);
export const ramp = (f: number, from: number, to: number) =>
  interpolate(f, [from, to], [0, 1], { ...clamp, easing: ease });

export function useSpring(at: number, damping = 18) {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return spring({ frame: frame - at, fps, config: { damping, mass: 0.7 } });
}

/** Fades and lifts in at a frame. */
export function Appear({
  at,
  children,
  style,
  dy = 12,
}: {
  at: number;
  children: ReactNode;
  style?: CSSProperties;
  dy?: number;
}) {
  const frame = useCurrentFrame();
  const p = ramp(frame, at, at + 10);
  return <div style={{ opacity: p, transform: `translateY(${(1 - p) * dy}px)`, ...style }}>{children}</div>;
}

export function TrafficLights({ size = 13 }: { size?: number }) {
  return (
    <div style={{ display: "flex", gap: size * 0.62 }}>
      {["#FF5F57", "#FEBC2E", "#28C840"].map((c) => (
        <div key={c} style={{ width: size, height: size, borderRadius: size, background: c }} />
      ))}
    </div>
  );
}

/** grafta's mark: two nodes and a line, from web/public/brand/mark.svg. */
export function Mark({ size, color = C.ink }: { size: number; color?: string }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64">
      <path d="M14 46 L32 18 L50 46" fill="none" stroke={color} strokeWidth={6} strokeLinecap="round" strokeLinejoin="round" />
      <circle cx={14} cy={46} r={8} fill={color} />
      <circle cx={32} cy={18} r={8} fill={color} />
      <circle cx={50} cy={46} r={8} fill={C.sap} />
    </svg>
  );
}

/** A plain mark for a design file: a page with a folded corner. */
export function FileGlyph({ size, color = C.deep, fill = C.soft }: { size: number; color?: string; fill?: string }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24">
      <path d="M6 2h8l5 5v13a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" fill={fill} stroke={color} strokeWidth={1.6} />
      <path d="M14 2v5h5" fill="none" stroke={color} strokeWidth={1.6} strokeLinejoin="round" />
    </svg>
  );
}
