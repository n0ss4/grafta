import { useEffect, useRef, useState } from "react";
import { href } from "~/lib/seo";
import type { Scene } from "~/lib/vtt";
import { GetGrafta } from "./GetGrafta";
import { SceneRail } from "./SceneRail";

const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * The first screen: the line and the button, then the video of grafta used from
 * a chat, ending in Figma, as large as the screen's height allows, with its
 * scenes under it. The requests, replies, code and pictures in it are a real
 * session's (grafta:video/src/talk.ts); the windows are drawn. Its scenes come
 * from its captions file (demo.vtt), written by the video's own code.
 */
export function Hero({ scenes }: { scenes: Scene[] }) {
  const video = useRef<HTMLVideoElement>(null);
  const [time, setTime] = useState(0);

  // Autoplay only when motion is welcome; the controls are there either way.
  // While it plays, the rail follows it frame by frame.
  useEffect(() => {
    const node = video.current;
    if (!node) return;
    let frame = 0;
    const follow = () => {
      setTime(node.currentTime);
      if (!node.paused) frame = requestAnimationFrame(follow);
    };
    const start = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(follow);
    };
    node.addEventListener("play", start);
    node.addEventListener("seeked", follow);
    if (reducedMotion()) node.pause();
    else void node.play().catch(() => undefined);
    return () => {
      cancelAnimationFrame(frame);
      node.removeEventListener("play", start);
      node.removeEventListener("seeked", follow);
    };
  }, []);

  const pick = (index: number) => {
    const node = video.current;
    const scene = scenes[index];
    if (!node || !scene) return;
    node.currentTime = scene.start + 0.01;
    setTime(node.currentTime);
    if (!reducedMotion()) void node.play().catch(() => undefined);
  };

  return (
    <header id="hero" className="hero px-4 pt-8 pb-12 sm:pt-10 lg:px-[6.25%]">
      <div className="grid gap-x-10 gap-y-4 lg:grid-cols-[auto_minmax(0,1fr)] lg:items-end">
        <h1 className="text-[38px] leading-[1.02] font-semibold tracking-[-0.035em] text-balance sm:text-[52px] lg:whitespace-nowrap">
          Talk to your Figma files.
        </h1>
        <div className="lg:justify-self-end">
          <p className="hero-support max-w-[540px] text-[16px] leading-relaxed text-muted sm:text-[17px]">
            Ask your AI what's in a file, change it, or turn it into code. No Figma account, no app running,
            no API limits.
          </p>
          <GetGrafta />
        </div>
      </div>

      <figure className="hero-stage mt-6 sm:mt-8">
        <div className="hero-frame">
          <video
            ref={video}
            className="block aspect-[2/1] h-auto w-full rounded-xl border border-line bg-sunk"
            poster={href("showcase/video/poster.jpg")}
            width={1920}
            height={960}
            muted
            loop
            playsInline
            controls
            preload="metadata"
            aria-label="grafta used from Claude Desktop, VS Code and Figma"
            aria-describedby="hero-video-scenes"
          >
            <source src={href("showcase/video/demo.webm")} type="video/webm" />
            <source src={href("showcase/video/demo.mp4")} type="video/mp4" />
            <track kind="captions" src={href("showcase/video/demo.vtt")} srcLang="en" label="English" />
          </video>
          <SceneRail scenes={scenes} time={time} onPick={pick} />
          <ol id="hero-video-scenes" className="sr-only">
            {scenes.map((scene) => (
              <li key={scene.name}>{scene.text}</li>
            ))}
          </ol>
        </div>
      </figure>
    </header>
  );
}
