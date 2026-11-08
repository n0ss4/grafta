import { AbsoluteFill, Img, staticFile, useCurrentFrame } from "remotion";
import { APPS } from "./talk";
import { AppIcon } from "./Dock";
import { Appear, C, MONO, T } from "./shared";

// ——— The end ———

export function EndCard() {
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
        <Appear at={at + 18} style={{ display: "flex", justifyContent: "center", gap: 26, marginTop: 44 }}>
          {APPS.map((a, i) => (
            <AppIcon key={a.name} i={i} size={56} />
          ))}
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

