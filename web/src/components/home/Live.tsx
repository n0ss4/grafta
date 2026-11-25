import type { ReactNode } from "react";
import { href } from "~/lib/seo";
import { Mark } from "../editorial/Mark";
import { Band, Checks, Head, NewBadge, Wrap } from "./Section";

// grafta's panel on the file open in Figma's desktop app: Figma's own window,
// in its light or dark interface, with the panel floating over the canvas as a
// plugin does. The panel's words are its own (figma-plugin/src/ui.html and
// activity.ts); the window is drawn.

const layers: [depth: number, glyph: string, name: string][] = [
  [0, "#", "Now Playing"],
  [1, "▭", "Cover"],
  [1, "T", "Golden Hour Drive"],
  [1, "T", "Lumen Coast"],
  [1, "▭", "Progress"],
  [1, "▭", "Play button"],
  [1, "✎", "Play"],
  [1, "✎", "Next"],
];

const points = [
  ["See it as it happens", "Each change shows on the canvas while it is made."],
  ["Before and after", "Each request shows how the frame was and how it is, layer by layer."],
  ["You give the go-ahead", "If you want, each change waits for you to apply it."],
  ["Undo anything", "A whole request goes back in one step of undo."],
] as const;

export function Live() {
  return (
    <Band id="live" labelledBy="live-title">
      <Wrap>
        <Head
          id="live-title"
          badge={<NewBadge className="mb-5" />}
          title="Live, in Figma"
          lead="Open grafta's panel in Figma, and the changes are made in the file you have open."
          rest="Each change shows as it is made. You see it before and after, and decide whether it stays."
        />
        <div
          role="img"
          aria-label="Figma with grafta's panel open: a request with its picture before and after"
          className="figma relative mt-16 overflow-hidden rounded-xl bg-[var(--f-canvas)] text-[12px] text-[var(--f-text)] shadow-[0_30px_60px_-30px_var(--shadow)] ring-1 ring-line md:h-[38.75rem]"
        >
          <FigmaBar />
          <aside className="absolute top-[46px] bottom-0 left-0 hidden w-[13.75rem] border-r border-[var(--f-line)] bg-[var(--f-panel)] lg:block">
            <Tabs first="File" second="Assets" />
            <p className="px-3.5 pt-3 pb-1.5 font-[510]">Layers</p>
            {layers.map(([depth, glyph, name]) => (
              <p
                key={name}
                className={`flex items-center gap-2 py-[5px] pr-3.5 whitespace-nowrap ${depth ? "pl-[30px]" : "pl-3.5"} ${
                  name === "Play button" ? "bg-[var(--f-sel)]" : ""
                }`}
              >
                <i className="w-3 text-center text-[11px] text-[var(--f-muted)] not-italic">{glyph}</i>
                {name}
              </p>
            ))}
          </aside>
          <div className="relative min-h-[23rem] [background-image:radial-gradient(rgb(128_128_128/0.14)_1px,transparent_1px)] [background-size:18px_18px] md:absolute md:top-[46px] md:right-0 md:bottom-0 md:left-0 lg:right-[14.375rem] lg:left-[13.75rem]">
            <div className="mx-auto w-[11.875rem] pt-10 md:absolute md:top-10 md:left-[9%] md:mx-0 md:pt-0">
              <p className="mb-1.5 text-[11px] text-[var(--f-muted)]">Now Playing</p>
              <div className="relative aspect-[360/680] overflow-hidden rounded-[2px] shadow-[0_0_0_1px_rgb(128_128_128/0.2)]">
                <img
                  src={href("showcase/player/after.webp")}
                  alt=""
                  width={480}
                  height={907}
                  loading="lazy"
                  className="absolute inset-0 size-full"
                />
                <span className="absolute top-[81.6%] left-[38.9%] h-[11.8%] w-[22.2%] rounded-[2px] border-[1.5px] border-accent-text" />
              </div>
            </div>
            <p className="mx-4 mt-6 mb-4 flex w-fit items-center gap-3 rounded-[9px] bg-[var(--f-panel)] px-3.5 py-2 text-[13px] shadow-[0_10px_30px_-12px_var(--shadow)] ring-1 ring-[var(--f-line)] md:absolute md:bottom-[26px] md:left-[4%] md:m-0">
              A change is waiting for you in grafta's panel
              <b className="font-[510] text-[var(--f-blue)]">Apply</b>
            </p>
          </div>
          <aside className="absolute top-[46px] right-0 bottom-0 hidden w-[14.375rem] border-l border-[var(--f-line)] bg-[var(--f-panel)] lg:block">
            <Tabs first="Design" second="Prototype" />
            <p className="px-3.5 pt-3 pb-1.5 font-[510]">Play button</p>
            <Prop name="W" value="72" />
            <Prop name="H" value="72" />
            <Prop name="Radius" value="36" />
            <div className="mt-2 border-t border-[var(--f-line)] pt-1">
              <p className="px-3.5 pt-3 pb-1.5 font-[510]">Fill</p>
              <Prop
                name="Accent"
                value={
                  <>
                    <i className="size-3.5 rounded-[3px] bg-[#ff6b4a]" />
                    100%
                  </>
                }
              />
            </div>
          </aside>
          <Panel />
        </div>
        <Checks items={points} className="mt-10" />
      </Wrap>
    </Band>
  );
}

export const figmaLogo = (
  <svg width="14" height="21" viewBox="0 0 38 57" aria-hidden="true">
    <path fill="#1abcfe" d="M19 28.5a9.5 9.5 0 1 1 19 0 9.5 9.5 0 0 1-19 0z" />
    <path fill="#0acf83" d="M0 47.5A9.5 9.5 0 0 1 9.5 38H19v9.5a9.5 9.5 0 1 1-19 0z" />
    <path fill="#ff7262" d="M19 0v19h9.5a9.5 9.5 0 1 0 0-19H19z" />
    <path fill="#f24e1e" d="M0 9.5A9.5 9.5 0 0 0 9.5 19H19V0H9.5A9.5 9.5 0 0 0 0 9.5z" />
    <path fill="#a259ff" d="M0 28.5A9.5 9.5 0 0 0 9.5 38H19V19H9.5A9.5 9.5 0 0 0 0 28.5z" />
  </svg>
);

const tools = [
  "M5 3l14 7-6 2-2 6z",
  "M7 3v18M17 3v18M3 7h18M3 17h18",
  "M4 4h16v16H4z",
  "M12 19l7-7 3 3-7 7zM18 13l-1.5-7.5L2 2l3.5 14.5L13 18zM2 2l7.6 7.6",
  "M5 5h14M12 5v14",
];

function FigmaBar() {
  return (
    <div className="relative flex h-[46px] items-center gap-1 border-b border-[var(--f-line)] bg-[var(--f-panel)] px-3 md:absolute md:inset-x-0 md:top-0">
      <span className="grid size-8 place-items-center">{figmaLogo}</span>
      {tools.map((d, i) => (
        <span
          key={d}
          className={`grid size-8 place-items-center rounded-md ${i === 0 ? "bg-[#0b74c4] text-white" : ""}`}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d={d} />
          </svg>
        </span>
      ))}
      <span className="absolute left-1/2 hidden -translate-x-1/2 text-[13px] text-[var(--f-muted)] md:inline">
        Drafts / <b className="font-[510] text-[var(--f-text)]">player</b>
      </span>
      <span className="ml-auto flex items-center gap-2.5">
        <span className="size-[26px] rounded-full bg-[linear-gradient(135deg,#f7a88a,#c46ad6)]" />
        <span className="rounded-md bg-[#0b74c4] px-3 py-1.5 font-[510] text-white">Share</span>
      </span>
    </div>
  );
}

function Tabs({ first, second }: { first: string; second: string }) {
  return (
    <p className="flex gap-3.5 border-b border-[var(--f-line)] px-3.5 py-3 font-[510]">
      {first}
      <span className="text-[var(--f-muted)]">{second}</span>
    </p>
  );
}

function Prop({ name, value }: { name: string; value: ReactNode }) {
  return (
    <p className="flex justify-between px-3.5 py-1.5 text-[var(--f-muted)]">
      {name}
      <span className="inline-flex items-center gap-1.5 text-[var(--f-text)]">{value}</span>
    </p>
  );
}

/** grafta's panel, as a plugin window over the canvas: one request, before and after. */
function Panel() {
  return (
    <div className="mx-4 mb-4 overflow-hidden rounded-xl bg-surface text-[13px] text-text shadow-[0_30px_60px_-20px_var(--shadow)] ring-1 ring-line md:absolute md:top-[70px] md:right-6 md:m-0 md:w-[22rem] lg:right-[15.625rem]">
      {/* Figma's own bar over a plugin's window, in Figma's colours */}
      <p className="flex h-9 items-center justify-between border-b border-[var(--f-line)] bg-[var(--f-panel)] px-3 text-[12px] font-[510] text-[var(--f-text)]">
        grafta
        <span aria-hidden="true" className="text-[15px] leading-none text-[var(--f-muted)]">
          ×
        </span>
      </p>
      <p className="flex items-center gap-2 border-b border-line px-3.5 py-3">
        <Mark size={16} />
        <b className="font-semibold">grafta</b>
        <span className="ml-auto rounded-full bg-soft px-2.5 py-0.5 text-[12px] font-[510] text-accent-text">
          Connected
        </span>
      </p>
      <div className="grid gap-3 p-3.5">
        <div className="grid gap-2.5 rounded-[10px] p-3 ring-1 ring-line">
          <p className="flex justify-between font-[510]">
            This request <span className="font-normal text-faint">10:42</span>
          </p>
          <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-1.5 text-faint">
            <Thumb label="Before" src="showcase/player/before.webp" />
            <span aria-hidden="true">→</span>
            <Thumb label="After" src="showcase/player/after.webp" after />
          </div>
          <div className="grid gap-1.5">
            <Change name="Corner radius" from="18" to="36" />
            <Change name="Fill" from="Ink" to="Accent" />
          </div>
          <div className="grid grid-cols-2 gap-2 text-[12.5px] whitespace-nowrap">
            <span className="flex h-8 items-center justify-center rounded-md ring-1 ring-line">
              Show all on canvas
            </span>
            <span className="flex h-8 items-center justify-center gap-1.5 rounded-md bg-[rgb(229_96_58/0.12)] text-limit">
              <span aria-hidden="true">↺</span>
              Revert this request
            </span>
          </div>
        </div>
        <p className="grid grid-cols-2 gap-0.5 rounded-lg bg-raised p-[3px] text-[12.5px] text-muted">
          <span className="flex h-7 items-center justify-center rounded-md">Apply directly</span>
          <span className="flex h-7 items-center justify-center rounded-md bg-surface text-text ring-1 ring-line">
            Ask me first
          </span>
        </p>
      </div>
    </div>
  );
}

function Thumb({ label, src, after = false }: { label: string; src: string; after?: boolean }) {
  return (
    <figure
      className={`relative grid h-[110px] place-items-center rounded-lg ${after ? "bg-soft ring-1 ring-accent/40 ring-inset" : "bg-raised"}`}
    >
      <figcaption
        className={`absolute top-1.5 left-2 text-[10.5px] ${after ? "text-accent-text" : "text-faint"}`}
      >
        {label}
      </figcaption>
      <img
        src={href(src)}
        alt=""
        width={480}
        height={907}
        loading="lazy"
        className="mt-3 h-[82px] w-auto rounded-[2px]"
      />
    </figure>
  );
}

function Change({ name, from, to }: { name: string; from: string; to: string }) {
  return (
    <p className="grid grid-cols-[104px_1fr] items-baseline gap-2 text-muted">
      {name}
      <span className="text-text">
        <s className="text-faint">{from}</s> → {to}
      </span>
    </p>
  );
}
