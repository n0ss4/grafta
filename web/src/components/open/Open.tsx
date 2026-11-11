import { useEffect, useReducer, useRef, useState } from "react";
import { CopyButton } from "~/components/install/CopyButton";
import { cn } from "~/lib/cn";
import { href } from "~/lib/seo";
import type { Summary } from "./protocol";
import { Reader, type ReaderError } from "./reader";
import { Screens } from "./Screens";
import { megabytes, productName, reduce } from "./state";
import { Tokens } from "./Tokens";

const SAMPLE = "order.fig";
/** What the input offers to choose; grafta decides what a file is, whatever its name. */
const ACCEPT = ".fig,.deck,.jam,.site";
const FONTS = ".ttf,.otf,.ttc,.otc";
/** The reader and its fonts, as built on 2026-11-11 (gzip): said before the first file. */
const FIRST_DOWNLOAD = "2.3 MB";

/** Open a Figma file in this page: drop it, choose it, or open the sample. */
export function Open() {
  // Made on the first render and kept; it starts its worker only when first asked.
  const [reader] = useState(() => new Reader());
  const [state, dispatch] = useReducer(reduce, { kind: "empty" });
  const [over, setOver] = useState(false);
  const [tab, setTab] = useState<"screens" | "tokens">("screens");
  const [fonts, setFonts] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const fontInput = useRef<HTMLInputElement>(null);
  const chooseRef = useRef<(file: File | undefined) => void>(() => {});

  useEffect(() => {
    const unlisten = reader.whenReady(() => {
      dispatch({ type: "ready" });
    });
    return () => {
      unlisten();
      reader.stop();
    };
  }, [reader]);

  // A file dropped anywhere on the page opens, as on the canvas of a design app.
  useEffect(() => {
    const files = (e: DragEvent) => Array.from(e.dataTransfer?.types ?? []).includes("Files");
    const onOver = (e: DragEvent) => {
      if (!files(e)) return;
      e.preventDefault();
      setOver(true);
    };
    const onLeave = (e: DragEvent) => {
      if (e.relatedTarget === null) setOver(false);
    };
    const onDrop = (e: DragEvent) => {
      if (!files(e)) return;
      e.preventDefault();
      setOver(false);
      chooseRef.current(e.dataTransfer?.files[0]);
    };
    window.addEventListener("dragover", onOver);
    window.addEventListener("dragleave", onLeave);
    window.addEventListener("drop", onDrop);
    return () => {
      window.removeEventListener("dragover", onOver);
      window.removeEventListener("dragleave", onLeave);
      window.removeEventListener("drop", onDrop);
    };
  }, []);

  async function read(name: string, size: number, bytes: () => Promise<ArrayBuffer>) {
    const r = reader;
    r.stop("Another file was chosen.");
    setTab("screens");
    setFonts(0);
    dispatch({ type: "chose", name, bytes: size, readerReady: false });
    if (size === 0 || size > 512 * 1024 * 1024) return;
    try {
      const summary = (await r.openBytes(name, await bytes())) as Summary;
      dispatch({ type: "opened", summary });
    } catch (caught) {
      const error = caught as ReaderError;
      if (error.message !== "Stopped." && error.message !== "Another file was chosen.") {
        dispatch({ type: "failed", message: refusal(name, error.message) });
      }
    }
  }

  function choose(file: File | undefined) {
    if (file) void read(file.name, file.size, () => file.arrayBuffer());
  }
  useEffect(() => {
    chooseRef.current = choose;
  });

  function sample() {
    void read(SAMPLE, 1, async () => (await fetch(href(`sample/${SAMPLE}`))).arrayBuffer());
  }

  function stop() {
    reader.stop();
    dispatch({ type: "stopped" });
  }

  async function addFonts(files: FileList | null) {
    if (!files) return;
    let faces = 0;
    for (const file of Array.from(files)) faces += await reader.font(await file.arrayBuffer());
    if (faces > 0) setFonts((n) => n + faces);
  }

  const busy = state.kind === "loading" || state.kind === "reading";

  return (
    <div>
      <input
        ref={input}
        type="file"
        accept={ACCEPT}
        className="sr-only"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(e) => {
          choose(e.target.files?.[0]);
          e.target.value = "";
        }}
      />

      {state.kind === "read" ? (
        <OpenFile
          summary={state.summary}
          reader={reader}
          tab={tab}
          setTab={setTab}
          fonts={fonts}
          onAnother={() => input.current?.click()}
          onFonts={() => fontInput.current?.click()}
        />
      ) : (
        <div
          className={cn(
            "flex min-h-[22rem] flex-col items-center justify-center rounded-xl border-2 border-dashed px-6 py-12 text-center transition-colors motion-reduce:transition-none",
            over ? "border-accent bg-soft" : "border-line bg-surface/40",
          )}
        >
          {busy ? (
            <div role="status" aria-live="polite">
              <p className="text-[19px] font-semibold">
                {state.kind === "loading" ? "Getting grafta ready" : `Reading ${state.name}`}
              </p>
              <p className="mt-2 text-[15px] text-muted">
                {state.kind === "loading"
                  ? `${FIRST_DOWNLOAD}, once: grafta runs in this page, so the file never leaves it.`
                  : "A large file takes a few seconds; one of hundreds of thousands of layers, longer."}
              </p>
              <button
                type="button"
                onClick={stop}
                className="mt-6 cursor-pointer rounded-lg border border-line px-3 py-1.5 text-[14px] font-semibold hover:border-text"
              >
                Stop
              </button>
            </div>
          ) : (
            <>
              <p className="text-[24px] font-semibold tracking-[-0.01em]">Drop a Figma file here</p>
              <p className="mt-2 text-[15px] text-muted">
                A .fig, .deck, .jam or .site, up to {megabytes(512 * 1024 * 1024)}.
              </p>
              <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
                <button
                  type="button"
                  onClick={() => input.current?.click()}
                  className="cursor-pointer rounded-lg bg-button px-4 py-2 text-[15px] font-semibold text-on-button hover:opacity-90"
                >
                  Choose a file
                </button>
                <button
                  type="button"
                  onClick={sample}
                  className="cursor-pointer rounded-lg border border-line px-4 py-2 text-[15px] font-semibold hover:border-text"
                >
                  Open the sample, {SAMPLE}
                </button>
              </div>
              {state.kind === "refused" && (
                <p role="alert" className="mt-6 max-w-[34rem] text-[15px] text-limit">
                  {state.message}
                </p>
              )}
            </>
          )}
        </div>
      )}

      <input
        ref={fontInput}
        type="file"
        accept={FONTS}
        multiple
        className="sr-only"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(e) => {
          void addFonts(e.target.files);
          e.target.value = "";
        }}
      />
    </div>
  );
}

/** grafta's refusal, with the file named and what to do. */
function refusal(name: string, message: string): string {
  return `grafta could not open ${name}: ${message}`;
}

function OpenFile({
  summary,
  reader,
  tab,
  setTab,
  fonts,
  onAnother,
  onFonts,
}: {
  summary: Summary;
  reader: Reader;
  tab: "screens" | "tokens";
  setTab: (tab: "screens" | "tokens") => void;
  fonts: number;
  onAnother: () => void;
  onFonts: () => void;
}) {
  return (
    <div>
      <div className="flex flex-wrap items-end justify-between gap-4 border-b border-line pb-4">
        <div className="min-w-0">
          <h2 className="truncate text-[28px] font-semibold tracking-[-0.02em]">{summary.file}</h2>
          <p className="mt-1 text-[14px] text-muted">
            {productName(summary.product)}, {summary.nodes.toLocaleString("en")} layers. Read in this page;
            nothing was sent.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={onFonts}
            className="cursor-pointer text-[14px] font-medium text-muted hover:text-text"
            title="grafta draws in Inter; add the file's own fonts to draw its text in them"
          >
            {fonts > 0 ? `Fonts added (${fonts})` : "Add fonts"}
          </button>
          <button
            type="button"
            onClick={onAnother}
            className="cursor-pointer rounded-lg border border-line px-3 py-1.5 text-[14px] font-semibold hover:border-text"
          >
            Open another file
          </button>
        </div>
      </div>

      <div role="tablist" aria-label="What to see" className="mt-4 flex gap-1">
        {(["screens", "tokens"] as const).map((t) => (
          <button
            key={t}
            role="tab"
            type="button"
            id={`tab-${t}`}
            aria-selected={tab === t}
            aria-controls={`panel-${t}`}
            onClick={() => {
              setTab(t);
            }}
            className={cn(
              "cursor-pointer rounded-md px-3 py-1.5 text-[15px]",
              tab === t ? "bg-soft font-semibold text-accent-text" : "text-text hover:bg-sunk",
            )}
          >
            {t === "screens" ? "Screens" : `Tokens (${summary.variables})`}
          </button>
        ))}
      </div>

      <div role="tabpanel" id={`panel-${tab}`} aria-labelledby={`tab-${tab}`} className="mt-6">
        {tab === "screens" ? (
          <Screens reader={reader} summary={summary} fonts={fonts} />
        ) : (
          <Tokens reader={reader} summary={summary} />
        )}
      </div>

      <OfferGrafta file={summary.file} />
    </div>
  );
}

/** The file in an AI app: grafta, and a first request naming it. */
export function OfferGrafta({ file }: { file: string }) {
  const request = `Using grafta, what's in ${file}?`;
  return (
    <section aria-labelledby="offer-title" className="mt-16 rounded-lg border border-line bg-surface p-6">
      <h2 id="offer-title" className="text-[22px] font-semibold tracking-[-0.01em]">
        Ask your AI about {file}
      </h2>
      <p className="mt-2 max-w-[40rem] text-[15px] text-muted">
        With grafta in your AI app, it reads the same file, changes it, or turns it into code in whatever
        language you work in.
      </p>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <code className="rounded-md bg-code px-3 py-1.5 text-[14px]">{request}</code>
        <CopyButton text={request} label="Copy the request" />
        <a href={href("#get")} className="text-[14px] font-semibold">
          Get grafta
        </a>
      </div>
    </section>
  );
}
