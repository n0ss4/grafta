import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "~/lib/cn";
import type { Caption, Screen, Screens as ScreensResult, Summary } from "./protocol";
import { type Reader, ReaderError } from "./reader";
import { save, zip } from "./save";
import { archiveName, pngName, unique, visiblePages } from "./state";

/** The longest side of a screen's picture in the grid, in pixels. */
const THUMB = 480;
/** The longest side of the selected screen's picture. */
const VIEW = 1600;
/** render_node's largest picture. */
const LARGEST = 4096;
const SCALES = [1, 2, 3] as const;
/** Screens asked for at once; the rest come with "Show more". */
const PAGE_SIZE = 60;

type Drawn = { url: string; caption: Caption } | { error: string } | "drawing";

/** The file's pages, and the chosen page's screens as frames on the canvas. */
export function Screens({ reader, summary, fonts }: { reader: Reader; summary: Summary; fonts: number }) {
  const { pages, internal } = visiblePages(summary);
  const [page, setPage] = useState(pages[0]?.id);
  const drawn = useDrawing(reader, summary.handle, fonts);

  if (!page) return <p className="text-muted">This file has no pages to show.</p>;

  return (
    <div className="grid gap-8 lg:grid-cols-[13rem_1fr]">
      <nav aria-label="Pages" className="lg:sticky lg:top-20 lg:self-start">
        <h3 className="micro mb-2 text-muted">Pages</h3>
        <ul className="flex gap-1 overflow-x-auto lg:flex-col">
          {pages.map((p) => (
            <li key={p.id}>
              <button
                type="button"
                aria-current={p.id === page ? "page" : undefined}
                onClick={() => {
                  setPage(p.id);
                }}
                className={cn(
                  "w-full cursor-pointer rounded-md px-2.5 py-1.5 text-left text-[14px] whitespace-nowrap",
                  p.id === page ? "bg-soft font-semibold text-accent-text" : "text-text hover:bg-sunk",
                )}
              >
                {p.name || "Untitled page"}
              </button>
            </li>
          ))}
        </ul>
        {internal > 0 && (
          <p className="mt-3 text-[13px] text-muted">
            {internal === 1
              ? "One page is Figma's own and not shown."
              : `${internal} pages are Figma's own and not shown.`}
          </p>
        )}
      </nav>
      {/* A page of its own: what it found and selected starts again with each page. */}
      <PageScreens key={page} reader={reader} summary={summary} page={page} fonts={fonts} drawn={drawn} />
    </div>
  );
}

/** A page's screens, drawn one at a time, those in view first. */
function PageScreens({
  reader,
  summary,
  page,
  fonts,
  drawn,
}: {
  reader: Reader;
  summary: Summary;
  page: string;
  fonts: number;
  drawn: ReturnType<typeof useDrawing>;
}) {
  const [screens, setScreens] = useState<ScreensResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Screen | null>(null);

  /** The next screens of a long page, after those shown. */
  const more = useCallback(
    async (cursor: string) => {
      try {
        const found = await reader.call<ScreensResult>("list_screens", {
          handle: summary.handle,
          page,
          limit: PAGE_SIZE,
          cursor,
        });
        setScreens((before) =>
          before ? { ...found, results: [...before.results, ...found.results] } : found,
        );
      } catch (caught) {
        setError((caught as ReaderError).message);
      }
    },
    [reader, summary.handle, page],
  );

  useEffect(() => {
    let live = true;
    reader
      .call<ScreensResult>("list_screens", { handle: summary.handle, page, limit: PAGE_SIZE })
      .then((found) => {
        if (live) setScreens(found);
      })
      .catch((caught: unknown) => {
        if (live) setError((caught as ReaderError).message);
      });
    return () => {
      live = false;
    };
  }, [reader, summary.handle, page]);

  return (
    <div className="min-w-0">
      {error && (
        <p role="alert" className="text-limit">
          {error}
        </p>
      )}
      {!screens && !error && <p className="text-muted">Finding the screens…</p>}
      {screens && (
        <>
          <PageActions reader={reader} summary={summary} screens={screens} />
          {selected && (
            <Selected
              key={selected.id}
              reader={reader}
              summary={summary}
              screen={selected}
              fonts={fonts}
              onClose={() => {
                setSelected(null);
              }}
            />
          )}
          {screens.results.length === 0 ? (
            <p className="mt-6 text-muted">Nothing is on this page.</p>
          ) : (
            <ul className="mt-6 grid grid-cols-[repeat(auto-fill,minmax(min(100%,15rem),1fr))] gap-x-6 gap-y-8">
              {screens.results.map((screen) => (
                <Frame
                  key={screen.id}
                  screen={screen}
                  drawn={drawn.state[screen.id]}
                  selected={screen.id === selected?.id}
                  onSeen={drawn.want}
                  onPick={() => {
                    setSelected(screen);
                  }}
                />
              ))}
            </ul>
          )}
          {screens.next_cursor && (
            <button
              type="button"
              onClick={() => {
                if (screens.next_cursor) void more(screens.next_cursor);
              }}
              className="mt-8 cursor-pointer rounded-lg border border-line px-3 py-1.5 text-[14px] font-semibold hover:border-text"
            >
              Show more ({screens.total - screens.results.length} left)
            </button>
          )}
        </>
      )}
    </div>
  );
}

/** One screen as a frame: its name above, as Figma shows it, and grafta's picture. */
function Frame({
  screen,
  drawn,
  selected,
  onSeen,
  onPick,
}: {
  screen: Screen;
  drawn: Drawn | undefined;
  selected: boolean;
  onSeen: (id: string) => void;
  onPick: () => void;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          onSeen(screen.id);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [screen.id, onSeen]);

  const ratio = screen.width > 0 && screen.height > 0 ? `${screen.width} / ${screen.height}` : "4 / 3";
  return (
    <li className="min-w-0">
      <p className="mb-1.5 truncate text-[13px] text-muted" title={screen.name}>
        {screen.name || "Untitled"}
        {!screen.visible && <span>, hidden in the file</span>}
      </p>
      <button
        ref={ref}
        type="button"
        onClick={onPick}
        aria-pressed={selected}
        aria-label={`${screen.name || "Untitled"}, ${Math.round(screen.width)} by ${Math.round(screen.height)}`}
        className={cn(
          "flex max-h-80 w-full cursor-pointer items-center justify-center overflow-hidden rounded-sm bg-surface outline-offset-2",
          selected ? "outline-2 outline-accent" : "outline-1 outline-line hover:outline-text",
          !screen.visible && "opacity-60",
        )}
        style={{ aspectRatio: ratio }}
      >
        {drawn && typeof drawn === "object" && "url" in drawn ? (
          <img src={drawn.url} alt="" className="h-full w-full object-contain" />
        ) : (
          <span className="px-3 text-center text-[13px] text-muted">
            {drawn && typeof drawn === "object" ? drawn.error : drawn === "drawing" ? "Drawing…" : ""}
          </span>
        )}
      </button>
    </li>
  );
}

/**
 * Pictures for the grid, drawn one at a time in the order screens came into
 * view: a slide with images takes seconds in a browser, and the reader does
 * one thing at once. Drawn again when fonts are added.
 */
function useDrawing(reader: Reader, handle: string, fonts: number) {
  const [state, setState] = useState<Record<string, Drawn>>({});
  const queue = useRef<string[]>([]);
  const busy = useRef(false);
  const generation = useRef(0);

  const pump = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    const mine = generation.current;
    while (queue.current.length > 0 && mine === generation.current) {
      const id = queue.current.shift();
      if (id === undefined) break;
      setState((s) => ({ ...s, [id]: "drawing" }));
      let result: Drawn;
      try {
        const picture = await reader.picture({ handle, id, max_size: THUMB });
        result = { url: URL.createObjectURL(picture.png), caption: picture.caption };
      } catch (caught) {
        const error = caught as ReaderError;
        if (error.fatal) {
          queue.current = [];
          busy.current = false;
          return;
        }
        result = { error: error.message.includes("no area") ? "Nothing to draw" : error.message };
      }
      if (mine === generation.current) setState((s) => ({ ...s, [id]: result }));
    }
    busy.current = false;
  }, [reader, handle]);

  const want = useCallback(
    (id: string) => {
      if (!queue.current.includes(id)) queue.current.push(id);
      void pump();
    },
    [pump],
  );

  useEffect(() => {
    if (fonts === 0) return;
    // New fonts: everything drawn so far is drawn again, in the same order.
    generation.current += 1;
    setState((s) => {
      queue.current = Object.keys(s);
      return {};
    });
    busy.current = false;
    void pump();
  }, [fonts, pump]);

  return { state, want };
}

/** The selected screen, larger, with what grafta drew approximately and its downloads. */
function Selected({
  reader,
  summary,
  screen,
  fonts,
  onClose,
}: {
  reader: Reader;
  summary: Summary;
  screen: Screen;
  fonts: number;
  onClose: () => void;
}) {
  const [picture, setPicture] = useState<{ url: string; caption: Caption } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [scale, setScale] = useState<(typeof SCALES)[number]>(2);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let live = true;
    reader
      .picture({ handle: summary.handle, id: screen.id, max_size: VIEW })
      .then((p) => {
        if (live) setPicture({ url: URL.createObjectURL(p.png), caption: p.caption });
      })
      .catch((caught: unknown) => {
        if (live) setError((caught as ReaderError).message);
      });
    return () => {
      live = false;
    };
  }, [reader, summary.handle, screen.id, fonts]);

  async function download() {
    setSaving(true);
    try {
      const p = await reader.picture({ handle: summary.handle, id: screen.id, scale, max_size: LARGEST });
      save(pngName(screen.name, scale), p.png);
    } catch (caught) {
      setError((caught as ReaderError).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <section aria-label={screen.name} className="mt-6 rounded-lg border border-line bg-surface p-4 sm:p-6">
      <div className="flex flex-wrap items-baseline justify-between gap-3">
        <h3 className="text-[19px] font-semibold">{screen.name || "Untitled"}</h3>
        <p className="text-[14px] text-muted">
          {screen.type.toLowerCase().replace(/_/g, " ")}, {Math.round(screen.width)} ×{" "}
          {Math.round(screen.height)}
        </p>
      </div>
      <div className="mt-4 flex max-h-[70vh] items-center justify-center overflow-auto rounded-sm bg-bg p-4">
        {picture ? (
          <img
            src={picture.url}
            alt={`${screen.name}, drawn by grafta`}
            className="max-h-[64vh] object-contain"
          />
        ) : (
          <p className="py-16 text-muted">{error ?? "Drawing…"}</p>
        )}
      </div>
      {picture && picture.caption.notes.length > 0 && (
        <div className="mt-4">
          <h4 className="text-[14px] font-semibold">Drawn approximately</h4>
          <ul className="mt-1 list-disc space-y-1 pl-5 text-[14px] text-muted">
            {picture.caption.notes.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </div>
      )}
      <div className="mt-5 flex flex-wrap items-center gap-3">
        <fieldset className="flex items-center gap-1 rounded-lg border border-line p-0.5">
          <legend className="sr-only">Scale</legend>
          {SCALES.map((s) => (
            <label
              key={s}
              className={cn(
                "cursor-pointer rounded-md px-2.5 py-1 text-[14px] font-medium",
                s === scale ? "bg-button text-on-button" : "text-text hover:bg-sunk",
              )}
            >
              <input
                type="radio"
                name="scale"
                value={s}
                checked={s === scale}
                onChange={() => {
                  setScale(s);
                }}
                className="sr-only"
              />
              {s}×
            </label>
          ))}
        </fieldset>
        <button
          type="button"
          disabled={saving}
          onClick={() => void download()}
          className="cursor-pointer rounded-lg bg-button px-3 py-1.5 text-[14px] font-semibold text-on-button hover:opacity-90 disabled:opacity-60"
        >
          {saving ? "Drawing the PNG…" : `Download PNG at ${scale}×`}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="cursor-pointer text-[14px] font-medium text-muted hover:text-text"
        >
          Close
        </button>
      </div>
    </section>
  );
}

/** A page's screens downloaded together, as one zip of PNGs. */
function PageActions({
  reader,
  summary,
  screens,
}: {
  reader: Reader;
  summary: Summary;
  screens: ScreensResult;
}) {
  const [progress, setProgress] = useState<string | null>(null);
  const drawable = screens.results.filter((s) => s.width > 0 && s.height > 0);

  async function downloadAll() {
    const files: { name: string; bytes: Uint8Array }[] = [];
    const skipped: string[] = [];
    for (const [i, screen] of drawable.entries()) {
      setProgress(`Drawing ${i + 1} of ${drawable.length}…`);
      try {
        const p = await reader.picture({
          handle: summary.handle,
          id: screen.id,
          scale: 1,
          max_size: LARGEST,
        });
        files.push({ name: pngName(screen.name, 1), bytes: new Uint8Array(await p.png.arrayBuffer()) });
      } catch (caught) {
        if ((caught as ReaderError).fatal) {
          setProgress((caught as ReaderError).message);
          return;
        }
        skipped.push(screen.name);
      }
    }
    const names = unique(files.map((f) => f.name));
    save(
      archiveName(summary.file, screens.page.name),
      await zip(files.map((f, i) => ({ ...f, name: names[i] ?? f.name }))),
    );
    setProgress(skipped.length ? `Saved. Not drawn: ${skipped.join(", ")}.` : null);
  }

  return (
    <div className="flex flex-wrap items-center gap-3">
      <p className="text-[14px] text-muted">
        {screens.total === 1 ? "1 screen" : `${screens.total} screens`} on {screens.page.name || "this page"}
        {screens.next_cursor ? `, ${screens.results.length} shown` : ""}.
      </p>
      {drawable.length > 1 && (
        <button
          type="button"
          disabled={progress !== null && progress.endsWith("…")}
          onClick={() => void downloadAll()}
          className="cursor-pointer rounded-lg border border-line px-3 py-1.5 text-[14px] font-semibold hover:border-text disabled:opacity-60"
        >
          Download {drawable.length} PNGs
        </button>
      )}
      {progress && (
        <p role="status" className="text-[14px] text-muted">
          {progress}
        </p>
      )}
    </div>
  );
}
