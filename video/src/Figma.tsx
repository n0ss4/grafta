import type { CSSProperties } from "react";
import { Img, staticFile, useCurrentFrame } from "remotion";
import { LAYERS } from "./talk";
import { C, MONO, T, TrafficLights, ramp, useSpring } from "./shared";

// ——— The result, open in Figma ———

export const FIG = { x: 180, y: 70, w: 1560, h: 800 };

export function FigmaWindow() {
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
      <div style={{ height: 48, background: "#fff", borderBottom: "1px solid #E6E6E6", display: "flex", alignItems: "center", gap: 14, padding: "0 16px", position: "relative" }}>
        <TrafficLights size={12} />
        <Img src={staticFile("logos/figma.png")} style={{ width: 30, marginLeft: 10 }} />
        <span style={{ fontSize: 14, color: "#8A8E95" }}>Drafts /</span>
        <span style={{ fontSize: 14, fontWeight: 600 }}>order-light</span>
        <span style={{ marginLeft: "auto", fontSize: 13, background: C.select, color: "#fff", borderRadius: 6, padding: "6px 14px", fontWeight: 600 }}>Share</span>
      </div>
      <div style={{ position: "absolute", top: 48, bottom: 0, left: 0, width: 300, background: "#fff", borderRight: "1px solid #E6E6E6" }}>
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

