import type { ReactNode } from "react";
import { GITHUB } from "~/content/site";
import { cn } from "~/lib/cn";
import { GitHubMark } from "../editorial/icons";
import { ClientMark } from "../install/ClientMark";
import { AppWindow } from "./AppWindow";
import { figmaLogo } from "./Live";
import { NewBadge, Wrap } from "./Section";

// The apps grafta works in, and Figma, floating round the line: a tile each,
// turned a little, on wide screens only. Where each sits is a share of the
// first screen, so they keep clear of the text.
const tiles: { mark: ReactNode; className: string }[] = [
  {
    mark: <ClientMark id="claude-desktop" size={34} />,
    className: "top-[38%] left-[3%] size-[5.5rem] -rotate-6",
  },
  { mark: <ClientMark id="cursor" size={24} />, className: "top-[6%] left-[13%] size-16 rotate-3" },
  { mark: <ClientMark id="chatgpt" size={24} />, className: "top-[82%] left-[12%] size-16 rotate-6" },
  {
    mark: <span className="scale-[1.6]">{figmaLogo}</span>,
    className: "top-[36%] right-[3%] size-[5.5rem] rotate-6",
  },
  { mark: <ClientMark id="opencode" size={24} />, className: "top-[6%] right-[13%] size-16 -rotate-3" },
  {
    mark: <ClientMark id="claude-code" size={26} />,
    className: "top-[82%] right-[12%] size-[4.5rem] -rotate-6",
  },
];

/**
 * The first screen: the line, what grafta is in one sentence, the way to get
 * it and the source, what is new, then the product at work.
 */
export function Hero() {
  return (
    <header id="hero" className="relative overflow-x-clip pt-[clamp(56px,8vw,104px)] pb-12">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-[46rem] [background-image:linear-gradient(var(--hair)_1px,transparent_1px),linear-gradient(90deg,var(--hair)_1px,transparent_1px)] [background-size:64px_64px] [mask-image:radial-gradient(ellipse_at_50%_30%,#000_20%,transparent_70%)]"
      />
      <Wrap className="relative">
        <div className="relative text-center">
          {tiles.map((tile, i) => (
            <span
              key={i}
              aria-hidden="true"
              className={cn(
                "absolute hidden place-items-center rounded-[22px] bg-surface shadow-[0_20px_40px_-16px_var(--shadow)] ring-1 ring-line xl:grid",
                tile.className,
              )}
            >
              {tile.mark}
            </span>
          ))}
          <a
            href="#live"
            className="inline-flex items-center gap-2.5 text-[14px] text-muted no-underline hover:text-text"
          >
            <NewBadge />
            Now in the Figma file you have open →
          </a>
          <h1 className="mx-auto mt-8 max-w-[14ch] text-[length:clamp(48px,7.4vw,92px)] leading-[0.98] font-semibold tracking-[-0.045em] text-balance">
            Talk to your Figma.
          </h1>
          <p className="mx-auto mt-7 max-w-[34rem] text-[length:clamp(17px,1.6vw,20px)] leading-[1.5] text-muted">
            <span className="text-text">grafta connects your AI to Figma.</span> Ask it anything about your
            designs.
          </p>
          <div className="mt-9 flex flex-col items-center gap-4">
            <a
              href="#get"
              className="inline-flex h-12 items-center rounded-[10px] bg-button px-7 text-[16px] font-semibold text-on-button no-underline hover:opacity-90"
            >
              Get grafta
            </a>
            <a
              href={GITHUB}
              className="inline-flex items-center gap-2 text-[15px] text-muted no-underline hover:text-text"
            >
              <GitHubMark size={16} />
              Free and open source, on GitHub
              <span aria-hidden="true">↗</span>
            </a>
          </div>
        </div>
        <AppWindow />
      </Wrap>
    </header>
  );
}
