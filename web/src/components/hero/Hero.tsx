import { useEffect, useRef } from "react";
import { href } from "~/lib/seo";
import { GetGrafta } from "./GetGrafta";

/** What the video shows, in words: for screen readers, search, and pages without video. */
export const SCENES = [
  "A Figma file, order.fig, is dragged into a chat in Claude Desktop, where grafta is connected.",
  "“What's in order.fig?” Claude reads its layers and variables through grafta, and describes the screen.",
  "“Make it light, and check the contrast.” It switches the file to its Light mode, checks every text against WCAG AA, and saves a new file, order-light.fig.",
  "“Now turn it into code.” The model writes order.html, opened in VS Code beside the page it draws.",
  "order-light.fig opens in Figma, with its layers and variables, to keep editing.",
];

/**
 * The first screen: the line, then a video of grafta used from a chat, ending
 * in Figma. The requests, replies, code and pictures in it are a real
 * session's (video/src/talk.ts); the windows are drawn.
 */
export function Hero() {
  const video = useRef<HTMLVideoElement>(null);

  // Autoplay only when motion is welcome; the controls are there either way.
  useEffect(() => {
    const node = video.current;
    if (!node) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) node.pause();
    else void node.play().catch(() => undefined);
  }, []);

  return (
    <header id="hero" className="px-4 pt-8 pb-14 sm:pt-12 lg:px-[6.25%] lg:pt-14 lg:pb-20">
      <div className="grid gap-x-10 gap-y-4 lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)] lg:items-end">
        <h1 className="text-[38px] leading-[1.02] font-semibold tracking-[-0.035em] text-balance sm:text-[56px] lg:text-[60px]">
          Talk to your Figma files.
        </h1>
        <div className="lg:pb-1.5">
          <p className="max-w-[520px] text-[16px] leading-relaxed text-muted sm:text-[17px]">
            Ask your AI what's in a file, change it, or turn it into code, in whatever language you work in.
            No Figma account, no Figma app running, no API limits.
          </p>
          <GetGrafta />
        </div>
      </div>

      <figure className="mt-8 sm:mt-10">
        <video
          ref={video}
          className="aspect-video h-auto w-full rounded-2xl border border-line bg-sunk"
          poster={href("showcase/video/poster.jpg")}
          width={1920}
          height={1080}
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
        </video>
        <figcaption className="mt-3 text-[13px] text-muted">
          48 seconds, no sound. The requests, replies, code and pictures are from a real session, shortened;
          the windows are drawn.
        </figcaption>
        <ol id="hero-video-scenes" className="sr-only">
          {SCENES.map((scene) => (
            <li key={scene}>{scene}</li>
          ))}
        </ol>
      </figure>
    </header>
  );
}
