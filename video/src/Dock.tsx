import { AbsoluteFill, Img, staticFile, useCurrentFrame } from "remotion";
import { APPS } from "./talk";
import { Appear, C, FileGlyph, T, WIDTH, ramp, useSpring } from "./shared";

export const DOCK = { y: 560, tile: 132, gap: 44 };
export const dockX = (i: number) => {
  const width = APPS.length * DOCK.tile + (APPS.length - 1) * DOCK.gap;
  return (WIDTH - width) / 2 + i * (DOCK.tile + DOCK.gap) + DOCK.tile / 2;
};

/** The apps grafta works in, each with its own mark; the first is clicked. */
export function Dock() {
  const frame = useCurrentFrame();
  const out = ramp(frame, T.open, T.open + 14);
  if (out >= 1) return null;
  return (
    <AbsoluteFill style={{ opacity: 1 - out, transform: `scale(${1 - out * 0.06})` }}>
      {/* shown from the very first frame, which is also what a paused video shows */}
      <div style={{ position: "absolute", top: 150, width: "100%", textAlign: "center" }}>
        <Img src={staticFile("brand/logo-light.svg")} style={{ height: 64 }} />
        <div style={{ fontSize: 76, fontWeight: 650, letterSpacing: "-0.035em", marginTop: 40 }}>Talk to your Figma files.</div>
        <div style={{ fontSize: 30, color: C.muted, marginTop: 18 }}>From the AI app you already use.</div>
      </div>
      {APPS.map((app, i) => (
        <AppTile key={app.name} i={i} />
      ))}
    </AbsoluteFill>
  );
}

export function AppIcon({ i, size }: { i: number; size: number }) {
  const app = APPS[i]!;
  if (!app.tile) {
    // a macOS app icon, as the app ships it: drawn with its own margin
    return <Img src={staticFile(app.logo)} style={{ width: size * 1.22, height: size * 1.22, margin: -size * 0.11 }} />;
  }
  const pad = ((app.pad ?? 26) / 132) * size;
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: size * 0.225,
        background: app.tile,
        display: "grid",
        placeItems: "center",
        boxShadow: "0 6px 18px rgb(15 17 21 / 0.18)",
        overflow: "hidden",
      }}
    >
      <Img
        src={staticFile(app.logo)}
        style={{
          width: size - pad * 2,
          height: size - pad * 2,
          filter: app.invert ? (app.pad === 0 ? "invert(1) contrast(1.4)" : "invert(1)") : undefined,
          transform: app.pad === 0 ? "scale(1.25)" : undefined,
        }}
      />
    </div>
  );
}

function AppTile({ i }: { i: number }) {
  const frame = useCurrentFrame();
  const app = APPS[i]!;
  // the apps are there from the first frame too, and settle in
  const pop = 0.85 + 0.15 * useSpring(i * 4, 13);
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
      <AppIcon i={i} size={DOCK.tile} />
      <div style={{ marginTop: 16, fontSize: 20, color: C.muted, whiteSpace: "nowrap", marginLeft: -40, marginRight: -40 }}>
        {app.name}
      </div>
    </div>
  );
}

// ——— The file on the desktop ———

export const FILE = { x: 158, y: 470 };

export function FigFile({ size = 1, name = "order.fig" }: { size?: number; name?: string }) {
  return (
    <div style={{ width: 96 * size, textAlign: "center" }}>
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
        <Img src={staticFile("logos/figma.png")} style={{ width: 46 * size }} />
      </div>
      <div style={{ marginTop: 10 * size, fontSize: 17 * size, fontWeight: 550 }}>{name}</div>
    </div>
  );
}

export { FileGlyph };
