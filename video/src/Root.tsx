import { AbsoluteFill, Composition } from "remotion";
import { DURATION, Demo, FPS, HEIGHT, WIDTH } from "./Demo";

/** The part of the scene the website shows: every window, at 2:1, without the captions' band. */
export const CROP = { x: 100, y: 52, w: 1700, h: 850 };
const WEB = { w: 1920, h: 960 };

function Web() {
  const scale = WEB.w / CROP.w;
  return (
    <AbsoluteFill style={{ overflow: "hidden" }}>
      <div
        style={{
          width: WIDTH,
          height: HEIGHT,
          transformOrigin: "0 0",
          transform: `scale(${scale}) translate(${-CROP.x}px, ${-CROP.y}px)`,
        }}
      >
        <Demo captions={false} />
      </div>
    </AbsoluteFill>
  );
}

export function Root() {
  return (
    <>
      {/* the website's: framed on the windows; its words are under it, on the page */}
      <Composition id="Web" component={Web} durationInFrames={DURATION} fps={FPS} width={WEB.w} height={WEB.h} />
      {/* for the README and posts: the whole frame, its captions in the picture */}
      <Composition id="Social" component={Demo} durationInFrames={DURATION} fps={FPS} width={WIDTH} height={HEIGHT} />
    </>
  );
}
