import { clients } from "~/content/clients";
import { FigmaMark } from "../editorial/icons";
import { Mark } from "../editorial/Mark";
import { ClientMark } from "../install/ClientMark";

/** A sparse, oversized diagram of grafta's core idea: an agent grafted into a local file. */
export function HeroCanvas() {
  return (
    <figure aria-labelledby="graft-caption" className="mx-auto mt-16 max-w-[1024px]">
      <figcaption id="graft-caption" className="sr-only">
        Your instruction and a local Figma file meet in grafta, producing a new editable Figma file.
      </figcaption>

      <div className="relative min-h-[620px] overflow-hidden rounded-[30px] border border-line bg-sunk sm:min-h-[680px] lg:min-h-[430px]">
        <div className="absolute inset-0 opacity-60 [background-image:radial-gradient(var(--line)_1px,transparent_1px)] [background-size:18px_18px]" />

        <svg
          aria-hidden="true"
          viewBox="0 0 1024 430"
          preserveAspectRatio="none"
          className="absolute inset-0 hidden size-full text-text/25 lg:block"
        >
          <path d="M140 324 512 86 884 324" fill="none" stroke="currentColor" strokeWidth="2" />
          <path
            d="M140 324 884 324"
            fill="none"
            stroke="currentColor"
            strokeWidth="1"
            strokeDasharray="5 8"
          />
        </svg>
        <div
          aria-hidden="true"
          className="absolute top-[90px] bottom-[100px] left-1/2 w-px bg-line lg:hidden"
        />

        <div className="absolute top-6 left-1/2 z-10 w-[calc(100%-48px)] max-w-[330px] -translate-x-1/2 lg:top-9">
          <div className="rounded-2xl border border-line bg-surface px-5 py-4 shadow-[0_12px_30px_rgb(21_23_28/0.08)]">
            <p className="font-mono text-[9px] tracking-[0.08em] text-muted uppercase">You ask</p>
            <p className="mt-1 text-[15px] leading-snug text-text">
              “Turn this web page into editable Figma layers.”
            </p>
          </div>
          <span className="mx-auto grid size-11 translate-y-[-1px] place-items-center rounded-full bg-ink text-paper ring-8 ring-sunk">
            <Mark size={25} />
          </span>
        </div>

        <div className="absolute top-[215px] left-1/2 z-10 w-[260px] -translate-x-1/2 lg:top-auto lg:bottom-7 lg:left-8 lg:translate-x-0">
          <p className="mb-2 flex items-center gap-2 font-mono text-[10px] text-muted">page.png</p>
          <div className="-rotate-2 rounded-xl border border-line bg-surface p-3 shadow-[0_18px_40px_rgb(21_23_28/0.10)]">
            <WebWireframe />
          </div>
          <p className="mt-3 text-[12px] text-muted">One flat page</p>
        </div>

        <div className="absolute right-1/2 bottom-7 z-10 w-[290px] translate-x-1/2 lg:right-8 lg:translate-x-0">
          <p className="mb-2 flex items-center justify-between font-mono text-[10px] text-deep-sap">
            <span className="flex items-center gap-2">
              <FigmaMark size={12} /> landing-editable.fig
            </span>
            <span>editable</span>
          </p>
          <div className="relative rotate-1 border border-sap bg-surface p-3 shadow-[0_22px_50px_rgb(21_23_28/0.14)]">
            {(["-top-1 -left-1", "-top-1 -right-1", "-bottom-1 -left-1", "-bottom-1 -right-1"] as const).map(
              (position) => (
                <i key={position} className={`absolute size-2 border border-sap bg-white ${position}`} />
              ),
            )}
            <span className="absolute bottom-full left-[-1px] bg-deep-sap px-1.5 py-0.5 font-mono text-[9px] text-white">
              Landing
            </span>
            <div className="grid grid-cols-[82px_1fr] overflow-hidden rounded-md bg-[#303030]">
              <div className="border-r border-white/10 p-2 font-mono text-[7px] text-white/70">
                <p className="text-white/70 uppercase">Layers</p>
                <ul className="mt-2 space-y-1.5">
                  <li>▣ Landing</li>
                  <li>◇ Navigation</li>
                  <li>◇ Hero</li>
                  <li>T Heading</li>
                  <li>◇ CTA</li>
                  <li>◇ Features</li>
                  <li>◇ Footer</li>
                </ul>
              </div>
              <div className="grid place-items-center p-2">
                <WebWireframe className="w-[158px] shadow-md" />
              </div>
            </div>
          </div>
          <p className="mt-3 text-right text-[12px] text-deep-sap">Frames, vectors and editable text</p>
        </div>
      </div>

      <div className="mt-5 flex flex-col gap-4 border-t border-line pt-5 sm:flex-row sm:items-center sm:justify-between">
        <p className="font-mono text-[10px] text-muted">One MCP · the AI app you already use</p>
        <ul className="flex flex-wrap items-center gap-x-5 gap-y-3">
          {clients.slice(0, 6).map((client) => (
            <li
              key={client.id}
              title={client.name}
              className="flex items-center gap-1.5 text-[10px] text-muted"
            >
              <ClientMark id={client.id} size={15} />
              <span className="hidden xl:inline">{client.name}</span>
            </li>
          ))}
        </ul>
      </div>
    </figure>
  );
}

function WebWireframe({ className = "mx-auto w-full" }: { className?: string }) {
  return (
    <div
      role="img"
      aria-label="A generic landing-page wireframe"
      className={`${className} aspect-[8/5] bg-white p-2 text-[#15171C]`}
    >
      <div className="flex items-center justify-between border-b border-[#15171C]/10 pb-1.5">
        <span className="size-2 rounded-full bg-[#15171C]/70" />
        <span className="flex gap-1">
          <i className="h-1 w-5 rounded bg-[#15171C]/20" />
          <i className="h-1 w-5 rounded bg-[#15171C]/20" />
          <i className="h-1 w-5 rounded bg-[#15171C]/20" />
        </span>
      </div>
      <div className="grid grid-cols-[1.15fr_0.85fr] gap-2 py-2">
        <span className="space-y-1.5">
          <i className="block h-2.5 w-4/5 rounded bg-[#15171C]/75" />
          <i className="block h-2.5 w-3/5 rounded bg-[#15171C]/75" />
          <i className="block h-1 w-full rounded bg-[#15171C]/15" />
          <i className="block h-1 w-4/5 rounded bg-[#15171C]/15" />
          <i className="block h-3 w-10 rounded bg-[#13704B]" />
        </span>
        <span className="rounded bg-[#E3F4EA]" />
      </div>
      <div className="grid grid-cols-3 gap-1.5 border-t border-[#15171C]/10 pt-2">
        {[0, 1, 2].map((item) => (
          <span key={item} className="space-y-1 rounded border border-[#15171C]/10 p-1.5">
            <i className="block size-2 rounded-sm bg-[#13704B]/35" />
            <i className="block h-1 w-full rounded bg-[#15171C]/20" />
            <i className="block h-1 w-3/4 rounded bg-[#15171C]/10" />
          </span>
        ))}
      </div>
    </div>
  );
}
