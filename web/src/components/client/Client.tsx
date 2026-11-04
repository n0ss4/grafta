import { useEffect, useRef, useState } from "react";
import { sessions, type SessionSpec } from "~/content/sessions";
import index from "~/generated/session-index.json";
import { cn } from "~/lib/cn";
import { useReducedMotion } from "~/lib/motion";
import { useTabs } from "../useTabs";
import { Design } from "./Design";
import { Fit } from "./Fit";
import { Desktop } from "./Desktop";
import { Terminal } from "./Terminal";
import { clientName, type ClientId, type Recording, type Turn } from "./types";
import type { View } from "./view";

const recorded = index as Record<string, ClientId[]>;
const NO_TURNS: Turn[] = [];

/** A turn index past any recording's end: the conversation shown whole. */
const WHOLE = Number.MAX_SAFE_INTEGER;

/** How long a turn is, in what play reveals: characters of text, or one step for a row of calls. */
const lengthOf = (turn: Turn | undefined) => (!turn ? 0 : turn.kind === "tools" ? 1 : turn.text.length);

/** The clients a session was recorded in, Claude Desktop first. */
function clientsOf(id: string): ClientId[] {
  const got = recorded[id] ?? [];
  return (["claude-desktop", "claude-code"] as const).filter((c) => got.includes(c));
}

/** The beat a recording has reached by turn `at`, as an index into the session's beats. */
function beatAt(spec: SessionSpec, turns: Turn[], at: number): number {
  let beat = -1;
  turns.slice(0, at + 1).forEach((t) => {
    if (t.kind === "tools" && !t.other)
      beat = Math.max(
        beat,
        spec.beats.findIndex((b) => b.id === t.beat),
      );
  });
  return beat;
}

function useCompact() {
  const [compact, setCompact] = useState(false);
  useEffect(() => {
    const query = window.matchMedia("(max-width: 639px)");
    const update = () => {
      setCompact(query.matches);
    };
    update();
    query.addEventListener("change", update);
    return () => {
      query.removeEventListener("change", update);
    };
  }, []);
  return compact;
}

/**
 * A person's request played in their AI client, as it was recorded:
 * the prompt typed and sent, the model's replies, grafta's work as rows that
 * open to the calls, and the Figma file beside it at each beat. Whole at build
 * time and under reduced motion; plays once on screen otherwise.
 */
export function Client({
  first: bundled,
  client: wanted,
  onClient,
}: {
  /** The session played on load, read at build time and pre-rendered. */
  first: Recording;
  /** The client the visitor last chose, here or in the install section. */
  client: string;
  onClient: (client: ClientId) => void;
}) {
  const reduce = useReducedMotion();
  const compact = useCompact();
  const [id, setId] = useState(bundled.id);
  const available = clientsOf(id);
  const client: ClientId = available.includes(wanted as ClientId)
    ? (wanted as ClientId)
    : (available[0] ?? bundled.client);
  const spec = sessions.find((s) => s.id === id) ?? sessions[0];
  const key = `${id}.${client}`;
  const [cache, setCache] = useState<Record<string, Recording>>({
    [`${bundled.id}.${bundled.client}`]: bundled,
  });
  const [failed, setFailed] = useState<string | null>(null);
  const recording = cache[key];
  const turns = recording?.turns ?? NO_TURNS;
  const last = turns.length - 1;

  const [at, setAt] = useState(WHOLE);
  const [chars, setChars] = useState(0);
  const [playing, setPlaying] = useState(false);
  const pausedByScroll = useRef(false);
  const started = useRef(false);
  const root = useRef<HTMLDivElement>(null);
  const logRef = useRef<HTMLDivElement>(null);

  // Any other session or client is fetched when picked.
  useEffect(() => {
    if (cache[key] || failed === key) return;
    let live = true;
    fetch(`${import.meta.env.BASE_URL}sessions/${key}.json`)
      .then((r) => (r.ok ? (r.json() as Promise<Recording>) : Promise.reject(new Error(String(r.status)))))
      .then((got) => {
        if (live) setCache((c) => ({ ...c, [key]: got }));
      })
      .catch(() => {
        if (live) setFailed(key);
      });
    return () => {
      live = false;
    };
  }, [key, cache, failed]);

  // Play: text appears a few characters at a time, a row of calls answers, a pause, the next turn.
  useEffect(() => {
    if (!playing || !recording) return;
    const turn = turns[at];
    if (!turn) return;
    const len = lengthOf(turn);
    let delay: number;
    let next: () => void;
    if (chars < len) {
      if (turn.kind === "tools") {
        delay = Math.min(1400, 300 + turn.calls.length * 120);
        next = () => {
          setChars(len);
        };
      } else {
        delay = 40;
        next = () => {
          setChars((c) => Math.min(len, c + (turn.kind === "user" ? 6 : 16)));
        };
      }
    } else {
      delay = turn.kind === "tools" ? 250 : 550;
      next = () => {
        if (at < last) {
          setAt(at + 1);
          setChars(0);
        } else setPlaying(false);
      };
    }
    const timer = setTimeout(next, delay);
    return () => {
      clearTimeout(timer);
    };
  }, [playing, recording, turns, at, chars, last]);

  // The conversation scrolls inside the client, never the page.
  useEffect(() => {
    if (!playing) return;
    const log = logRef.current;
    if (log) log.scrollTop = log.scrollHeight;
  }, [playing, at, chars]);

  const start = () => {
    setAt(0);
    setChars(0);
    setPlaying(true);
  };
  // Effects that outlive a render call the latest choose and start through these.
  const startRef = useRef(start);
  const chooseRef = useRef<(next: string) => void>(() => undefined);
  useEffect(() => {
    startRef.current = start;
    chooseRef.current = choose;
  });

  // Plays the first time it is on screen; pauses off screen and resumes on return.
  useEffect(() => {
    const node = root.current;
    if (!node || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry) return;
        if (entry.isIntersecting) {
          if (!started.current) {
            started.current = true;
            if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) startRef.current();
          } else if (pausedByScroll.current) {
            pausedByScroll.current = false;
            setPlaying(true);
          }
        } else {
          setPlaying((p) => {
            if (p) pausedByScroll.current = true;
            return false;
          });
        }
      },
      { threshold: 0.35 },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, []);

  function choose(next: string) {
    started.current = true;
    setId(next);
    setFailed(null);
    if (reduce) {
      setAt(WHOLE);
      setPlaying(false);
    } else start();
  }

  // A job in "What you can ask" plays its session here.
  useEffect(() => {
    const onSession = (event: Event) => {
      const next = (event as CustomEvent<string>).detail;
      if (sessions.some((s) => s.id === next)) {
        chooseRef.current(next);
        root.current?.scrollIntoView({ block: "center" });
      }
    };
    window.addEventListener("grafta:session", onSession);
    return () => {
      window.removeEventListener("grafta:session", onSession);
    };
  }, []);

  const switchTo = (next: ClientId) => {
    if (next === client || !spec) return;
    const beat = beatAt(spec, turns, at);
    onClient(next);
    const other = cache[`${id}.${next}`];
    if (!other) {
      setAt(WHOLE);
      setPlaying(false);
      return;
    }
    // The same beat in the other recording: its first row at or past it.
    const target =
      beat < 0
        ? 0
        : other.turns.findIndex(
            (t) => t.kind === "tools" && beatAt(spec, other.turns, other.turns.indexOf(t)) >= beat,
          );
    const to = target < 0 ? other.turns.length - 1 : target;
    setAt(to);
    setChars(lengthOf(other.turns[to]));
  };

  const tabs = useTabs(
    sessions.map((s) => s.id),
    id,
    choose,
  );

  if (!spec) return null;
  // WHOLE (or anything past the end) shows the conversation finished.
  const whole = at > last;
  const shownAt = Math.min(at, last);
  const turn = turns[shownAt];
  const finished = whole || chars >= lengthOf(turn);
  const designAt = turn?.kind === "tools" && !finished ? shownAt - 1 : shownAt;
  const beat = beatAt(spec, turns, designAt);
  let lastTools = -1;
  turns.slice(0, shownAt + 1).forEach((t, i) => {
    if (t.kind === "tools" && !t.other && (i < shownAt || finished)) lastTools = i;
  });
  const view: View = {
    turns: turns.slice(0, shownAt + 1),
    at: shownAt,
    chars: whole ? lengthOf(turn) : chars,
    finished,
    typing: shownAt === 0 && turn?.kind === "user" && !finished ? turn.text.slice(0, chars) : undefined,
    working: playing && !(shownAt === last && finished),
    lastTools,
  };
  const design = recording ? (
    <Design spec={spec} recording={recording} at={designAt} compact={compact} />
  ) : null;
  const done = shownAt >= last && finished;
  const title = spec.title;

  return (
    <div ref={root} className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div
          {...tabs.list("Requests")}
          className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-1 lg:mx-0 lg:flex-wrap lg:px-0"
        >
          {sessions.map((s, i) => (
            <button
              key={s.id}
              {...tabs.tab(s.id, i)}
              className="flex-none cursor-pointer rounded-full border border-line bg-surface/60 px-4 py-2 text-[14px] font-medium text-text hover:border-text data-[state=active]:border-text data-[state=active]:bg-text data-[state=active]:text-bg"
            >
              {s.title}
            </button>
          ))}
        </div>
        {available.length > 1 ? (
          <div
            role="radiogroup"
            aria-label="Client"
            className="flex w-fit gap-0.5 rounded-full border border-line bg-surface/70 p-[3px]"
          >
            {available.map((c) => (
              <button
                key={c}
                type="button"
                role="radio"
                aria-checked={c === client}
                onClick={() => {
                  switchTo(c);
                }}
                className={cn(
                  "cursor-pointer rounded-full px-3 py-1.5 text-[13px] font-medium",
                  c === client ? "bg-soft font-semibold text-accent-text" : "text-muted hover:text-text",
                )}
              >
                {clientName[c]}
              </button>
            ))}
          </div>
        ) : null}
      </div>

      {sessions.map((s) => (
        <div key={s.id} {...tabs.panel(s.id)}>
          {s.id === id ? (
            <div className="grid grid-cols-[minmax(0,1fr)] gap-4 lg:grid-cols-[minmax(0,640fr)_minmax(0,536fr)] lg:gap-6">
              <div className="h-[560px] sm:h-[620px]">
                {!recording ? (
                  <div className="grid h-full place-items-center rounded-2xl border border-line bg-surface text-[15px] text-muted">
                    {failed === key ? (
                      <p className="flex flex-col items-center gap-3">
                        This session did not load.
                        <button
                          type="button"
                          className="cursor-pointer rounded-full border border-line px-4 py-1.5 text-text"
                          onClick={() => {
                            setFailed(null);
                          }}
                        >
                          Try again
                        </button>
                      </p>
                    ) : (
                      "Loading the session…"
                    )}
                  </div>
                ) : client === "claude-code" ? (
                  <Terminal view={view} logRef={logRef} inline={compact ? design : undefined} />
                ) : (
                  <Desktop
                    view={view}
                    logRef={logRef}
                    title={title}
                    model={recording.model}
                    inline={compact ? design : undefined}
                  />
                )}
              </div>
              {!compact ? (
                <figure
                  aria-label={beat >= 0 ? spec.beats[beat]?.title : "The file, as opened"}
                  className="relative hidden h-[620px] flex-col overflow-hidden rounded-2xl bg-sunk sm:flex"
                >
                  <figcaption className="flex items-center justify-between gap-4 px-5 pt-4 text-[13px]">
                    <span className="flex min-w-0 items-center gap-2">
                      <span aria-hidden className="size-2.5 flex-none rounded-[2px] bg-sap" />
                      <span className="truncate font-medium">
                        {(spec.prompt.match(/[\w-]+\.fig/) ??
                          spec.prompt.match(/[\w-]+\.(?:png|jpe?g)/))?.[0] ?? ""}
                      </span>
                      {recording?.file && done ? (
                        <span className="truncate text-muted">→ {recording.file}</span>
                      ) : null}
                    </span>
                    <span aria-live="polite" className="flex-none font-medium text-accent-text">
                      {beat >= 0
                        ? `Step ${beat + 1} of ${spec.beats.length} · ${spec.beats[beat]?.title ?? ""}`
                        : "Opening"}
                    </span>
                  </figcaption>
                  <div className="min-h-0 flex-1">
                    <Fit>{design}</Fit>
                  </div>
                </figure>
              ) : null}
            </div>
          ) : null}
        </div>
      ))}

      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:gap-6">
        <div className="flex flex-none gap-1.5">
          {done ? (
            <Control label="Replay" onClick={start} />
          ) : (
            <Control
              label={playing ? "Pause" : "Play"}
              onClick={() => {
                pausedByScroll.current = false;
                setPlaying((p) => !p);
              }}
            />
          )}
          <Control
            label="Step"
            disabled={done}
            onClick={() => {
              setPlaying(false);
              if (!finished) setChars(lengthOf(turn));
              else if (shownAt < last) {
                setAt(shownAt + 1);
                setChars(lengthOf(turns[shownAt + 1]));
              }
            }}
          />
        </div>
        <p className="text-[14px] leading-snug font-medium">
          {spec.outcome.summary}
          {recording?.file ? <span className="text-accent-text"> · {recording.file}</span> : null}
        </p>
      </div>
      {recording ? <Provenance recording={recording} /> : null}
    </div>
  );
}

function Control({ label, onClick, disabled }: { label: string; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="cursor-pointer rounded-full border border-text/20 px-3.5 py-1.5 text-[13px] font-medium hover:border-text disabled:cursor-default disabled:opacity-40"
    >
      {label}
    </button>
  );
}

/** Where, when and with what the session was recorded, and what the site changed. */
function Provenance({ recording }: { recording: Recording }) {
  const { edits } = recording;
  const changed = [
    edits.paths && "paths shortened",
    edits.cut && `${edits.cut} long result${edits.cut === 1 ? "" : "s"} cut`,
    edits.otherTools &&
      `${edits.otherTools} call${edits.otherTools === 1 ? "" : "s"} to the client's own tools shown collapsed`,
    edits.pictures && `${edits.pictures} picture link${edits.pictures === 1 ? "" : "s"} pointed at the site`,
  ].filter(Boolean);
  return (
    <p className="text-[12px] leading-relaxed text-muted">
      Recorded in {clientName[recording.client]} {recording.clientVersion} with {recording.model} on{" "}
      {recording.recordedOn}. The window is redrawn; the words and calls are as recorded
      {changed.length ? `, except: ${changed.join("; ")}` : ""}.
    </p>
  );
}
