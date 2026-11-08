import type { CSSProperties } from "react";
import { Img, staticFile, useCurrentFrame } from "remotion";
import { LAYERS } from "./talk";
import { C, MONO, T, TrafficLights, figmaReady, ramp, useSpring } from "./shared";
import { FIG_ROW } from "./VSCode";

// ——— The result, open in Figma ———

export const FIG = { x: 180, y: 70, w: 1560, h: 800 };

export function FigmaWindow() {
  const frame = useCurrentFrame();
  const rise = useSpring(T.figma, 16);
  const F0 = figmaReady();
  const fromX = FIG_ROW.x - (FIG.x + FIG.w / 2);
  const fromY = FIG_ROW.y - (FIG.y + FIG.h / 2);
  const gone = ramp(frame, T.end, T.end + 12);
  if (frame < T.figma) return null;
  const selected = frame >= F0 + 64;
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
        transform: `translate(${(1 - rise) * fromX}px, ${(1 - rise) * fromY}px) scale(${0.1 + rise * 0.9})`,
      }}
    >
      <div style={{ height: 48, background: "#fff", borderBottom: "1px solid #E6E6E6", display: "flex", alignItems: "center", gap: 14, padding: "0 16px", position: "relative" }}>
        <TrafficLights size={12} />
        <Img src={staticFile("logos/figma.png")} style={{ width: 30, marginLeft: 10 }} />
        <span style={{ fontSize: 14, color: "#8A8E95" }}>Drafts /</span>
        <span style={{ fontSize: 14, fontWeight: 600 }}>order-light</span>
        <span style={{ marginLeft: "auto", fontSize: 13, background: C.select, color: "#fff", borderRadius: 6, padding: "6px 14px", fontWeight: 600 }}>Share</span>
      </div>
      <Loading />
      <div style={{ position: "absolute", top: 48, bottom: 0, left: 0, width: 300, background: "#fff", borderRight: "1px solid #E6E6E6" }}>
        <div style={{ padding: "16px 18px 10px", fontSize: 14, fontWeight: 650 }}>Layers</div>
        {LAYERS.map((layer, i) => {
          const p = ramp(frame, F0 + 20 + i * 3, F0 + 28 + i * 3);
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
      <div style={{ position: "absolute", top: 48, bottom: 0, right: 0, width: 300, background: "#fff", borderLeft: "1px solid #E6E6E6" }}>
        <FigPanel selected={selected} />
      </div>
      {/* the canvas */}
      <div style={{ position: "absolute", top: 48, bottom: 0, left: 300, right: 300, display: "grid", placeItems: "center" }}>
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
  const p = ramp(frame, figmaReady() + 70, figmaReady() + 82);
  const row: CSSProperties = { display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 14, padding: "7px 0" };
  if (!selected) return <div style={{ padding: 18, fontSize: 14, color: "#8A8E95" }}>Design</div>;
  return (
    <div style={{ padding: 18, opacity: p }}>
      <div style={{ fontSize: 14, fontWeight: 650, marginBottom: 14 }}>Frame · Order</div>
      <div style={{ fontSize: 14, fontWeight: 650, marginBottom: 6 }}>Auto layout</div>
      {[
        ["Direction", "Vertical"],
        ["Gap", "20"],
        ["Padding", "24"],
      ].map(([name, value]) => (
        <div key={name} style={row}>
          <span style={{ color: "#8A8E95" }}>{name}</span>
          <span>{value}</span>
        </div>
      ))}
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


/** Figma reading the file: its mark and the file's name, then the editor. */
function Loading() {
  const frame = useCurrentFrame();
  const veil = 1 - ramp(frame, figmaReady() - 4, figmaReady() + 8);
  if (veil <= 0) return null;
  const bar = ramp(frame, T.figma + 6, figmaReady());
  return (
    <div style={{ position: "absolute", top: 48, bottom: 0, left: 0, right: 0, zIndex: 5, background: "#F5F5F5", opacity: veil, display: "grid", placeItems: "center" }}>
      <div style={{ textAlign: "center" }}>
        <Img src={staticFile("logos/figma.png")} style={{ width: 96 }} />
        <div style={{ fontSize: 17, marginTop: 14, color: "#4A4E55" }}>Opening order-light.fig</div>
        <div style={{ width: 220, height: 4, borderRadius: 2, background: "#E2E2E2", margin: "16px auto 0", overflow: "hidden" }}>
          <div style={{ width: `${bar * 100}%`, height: "100%", background: C.select }} />
        </div>
      </div>
    </div>
  );
}
