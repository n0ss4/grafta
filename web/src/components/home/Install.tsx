import type { ReactNode } from "react";
import { firstRequest, panel, panelRequest } from "~/content/clients";
import { CopyButton } from "../install/CopyButton";
import { Installer, Line, useClient } from "../install/Installer";
import { Band, Head, Wrap } from "./Section";

/**
 * Getting grafta, on the page, in the order it is done: the AI app, then
 * grafta's panel in Figma for the file you have open, then the first request.
 * Every step is here, the panel's included, as docs/guides/in-figma.md has it.
 */
export function Install() {
  return (
    <Band id="get" labelledBy="get-title">
      <Wrap>
        <Head
          id="get-title"
          title="Get grafta in three steps"
          lead="Add it to your AI app, then to Figma, then ask."
          rest="The second step is for the file you have open in Figma. Skip it to work on files on your computer."
        />
        <ol className="mt-14 border-t border-line">
          <Step
            number={1}
            title="Add grafta to your AI app"
            aside="Pick the app you use. For macOS, Linux and Windows."
          >
            <Installer />
          </Step>
          <Step
            number={2}
            title="Add grafta's panel to Figma"
            aside="For the file you have open in Figma's desktop app. Once per computer."
          >
            <Panel />
          </Step>
          <Step number={3} title="Ask" aside="In words, in your AI app.">
            <Try />
          </Step>
        </ol>
      </Wrap>
    </Band>
  );
}

function Step({
  number,
  title,
  aside,
  children,
}: {
  number: number;
  title: string;
  aside: string;
  children: ReactNode;
}) {
  return (
    <li className="border-b border-line py-10 last:border-b-0 last:pb-0">
      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
        <h3 className="flex items-baseline gap-3 text-[22px] leading-tight font-semibold tracking-[-0.02em]">
          <span className="font-mono text-[14px] font-normal text-faint">{number}</span>
          {title}
        </h3>
        <p className="text-[15px] text-muted">{aside}</p>
      </div>
      <div className="mt-6 min-w-0">{children}</div>
    </li>
  );
}

/** A path through Figma's menus, each item as it reads there. */
function Path({ items }: { items: string[] }) {
  return (
    <span className="inline-flex flex-wrap items-center gap-1 align-middle">
      {items.map((item, i) => (
        <span key={item} className="inline-flex items-center gap-1">
          {i > 0 ? (
            <span aria-hidden="true" className="text-faint">
              ›
            </span>
          ) : null}
          <kbd className="rounded-md bg-surface px-1.5 py-0.5 font-sans text-[13.5px] font-[510] text-text ring-1 ring-line">
            {item}
          </kbd>
        </span>
      ))}
    </span>
  );
}

const IMPORT = ["Plugins", "Development", "Import plugin from manifest…"];

/** The panel's four steps, and Figma's menus drawn along the path to import it. */
function Panel() {
  const client = useClient();
  return (
    <div className="grid gap-8 xl:grid-cols-[minmax(0,1fr)_16rem]">
      <ol className="grid gap-5 text-[15px] leading-relaxed">
        <Sub n="a">
          Download the panel and unzip it.
          <Line className="mt-2.5">
            <span className="min-w-0 truncate font-mono text-[14px]">{panel.file}</span>
            <a
              href={panel.href}
              className="flex-none rounded-lg bg-button px-3.5 py-1.5 text-[14px] font-semibold text-on-button no-underline hover:opacity-90"
            >
              Download
            </a>
          </Line>
        </Sub>
        <Sub n="b">
          In Figma's desktop app, choose <Path items={IMPORT} /> and pick the{" "}
          <code className="font-mono text-[14px]">manifest.json</code> in the folder you unzipped.
        </Sub>
        <Sub n="c">
          Open a file, then <Path items={["Plugins", "Development", "grafta"]} />. The panel opens and looks
          for grafta on your computer.
        </Sub>
        <Sub n="d">
          It finds grafta while your AI app runs it, and asks: “
          {client?.group === "app" ? client.name : "Your app"} wants to use this file”. Click{" "}
          <b className="font-semibold">Allow</b>. It asks once per app.
        </Sub>
      </ol>
      <FigmaMenus />
    </div>
  );
}

function Sub({ n, children }: { n: string; children: ReactNode }) {
  return (
    <li className="grid grid-cols-[20px_minmax(0,1fr)] gap-2">
      <span aria-hidden="true" className="pt-px font-mono text-[13px] text-faint">
        {n}
      </span>
      <div className="min-w-0">{children}</div>
    </li>
  );
}

/**
 * Figma's menus, opened along step b's path: only the items on it are named,
 * so the drawing says no more than the step does.
 */
function FigmaMenus() {
  const menu =
    "figma w-max rounded-lg bg-[var(--f-panel)] p-1 text-[12.5px] text-[var(--f-text)] shadow-[0_12px_30px_-10px_var(--shadow)] ring-1 ring-[var(--f-line)]";
  const item = "flex items-center justify-between gap-6 rounded px-2.5 py-1.5";
  const on = `${item} bg-[#0b74c4] text-white`;
  return (
    <div
      role="img"
      aria-label={`Figma's menus open along ${IMPORT.join(", then ")}`}
      className="relative hidden h-[11.5rem] xl:block"
    >
      <div className={`${menu} absolute top-0 left-0`}>
        <p className={on}>
          Plugins <span aria-hidden="true">›</span>
        </p>
      </div>
      <div className={`${menu} absolute top-10 left-6`}>
        <p className={on}>
          Development <span aria-hidden="true">›</span>
        </p>
      </div>
      <div className={`${menu} absolute top-20 left-12`}>
        <p className={on}>Import plugin from manifest…</p>
      </div>
      <div className="absolute top-[7.75rem] left-12 flex items-center gap-2 rounded-lg bg-surface px-3 py-2 text-[12.5px] ring-1 ring-line">
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          aria-hidden="true"
          className="text-faint"
        >
          <path d="M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z" />
          <path d="M14 3v5h5" />
        </svg>
        <span className="font-mono">manifest.json</span>
      </div>
    </div>
  );
}

/** The first requests: on the file open in Figma, or on a file saved from it. */
function Try() {
  const client = useClient();
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <section aria-label="Try it in Figma" className="rounded-xl bg-surface p-5 ring-1 ring-line">
        <h4 className="text-[15px] font-[510]">On the file open in Figma</h4>
        <p className="mt-1 text-[14px] text-muted">Select a layer, with the panel open, and ask:</p>
        <Request text={panelRequest} />
      </section>
      <section aria-label="Try it on a Figma file" className="rounded-xl bg-surface p-5 ring-1 ring-line">
        <h4 className="text-[15px] font-[510]">On a file on your computer</h4>
        <p className="mt-1 text-[14px] text-muted">
          In Figma, File → Save local copy. Then ask, with the file's name:
        </p>
        <Request text={firstRequest} />
        {client?.tryNote ? (
          <p className="mt-3 text-[13px] leading-relaxed text-muted">{client.tryNote}</p>
        ) : null}
      </section>
    </div>
  );
}

function Request({ text }: { text: string }) {
  return (
    <div className="mt-3 flex items-center justify-between gap-3 rounded-[10px] bg-raised px-4 py-3 ring-1 ring-line">
      <span className="min-w-0 text-[15px]">“{text}”</span>
      <CopyButton text={text} label="Copy" />
    </div>
  );
}
