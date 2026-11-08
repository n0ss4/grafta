import { Composition } from "remotion";
import { DURATION, Demo, FPS, HEIGHT, WIDTH } from "./Demo";

export function Root() {
  return <Composition id="Demo" component={Demo} durationInFrames={DURATION} fps={FPS} width={WIDTH} height={HEIGHT} />;
}
