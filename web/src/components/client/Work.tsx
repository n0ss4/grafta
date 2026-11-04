import { useId, useState, type ReactNode } from "react";
import { cn } from "~/lib/cn";
import { ToolRow } from "./ToolRow";
import type { Step, Turn } from "./types";

type Tools = Extract<Turn, { kind: "tools" }>;

/** A row of a run of work, with where it sits in the conversation. */
export interface WorkRow {
  turn: Tools;
  index: number;
  running: boolean;
}

/** What the conversation shows in order: the person's turn, a reply, or a run of work between them. */
export type Block =
  | { kind: "user" | "reply"; index: number; turn: Extract<Turn, { kind: "user" | "reply" }> }
  | { kind: "work"; rows: WorkRow[] };

/** Turns as the clients show them: consecutive rows of calls folded into one run. */
export function blocks(turns: Turn[], at: number, finished: boolean): Block[] {
  const out: Block[] = [];
  turns.forEach((turn, index) => {
    if (turn.kind !== "tools") {
      out.push({ kind: turn.kind, index, turn });
      return;
    }
    const row = { turn, index, running: index === at && !finished };
    const last = out.at(-1);
    if (last?.kind === "work") last.rows.push(row);
    else out.push({ kind: "work", rows: [row] });
  });
  return out;
}

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/**
 * A run of work between two replies, folded as the clients fold it: Claude
 * shows "Used grafta and 11 more steps" with the pictures it made, Claude Code
 * "Called grafta 12 times". Open, it lists each row of calls; each row opens
 * to its calls as recorded.
 */
export function Work({
  rows,
  tone,
  after,
}: {
  rows: WorkRow[];
  tone: "desktop" | "terminal";
  /** Drawn under the run, open or not: the design inline on a phone. */
  after?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const calls: Step[] = rows.flatMap((r) => r.turn.calls);
  const grafta = rows.filter((r) => !r.turn.other).reduce((n, r) => n + r.turn.calls.length, 0);
  const others = calls.length - grafta;
  const running = rows.find((r) => r.running);
  const pictures = calls.flatMap((c) => (c.picture ? [c.picture] : []));
  const terminal = tone === "terminal";

  const summary = running
    ? `${running.turn.label}…`
    : terminal
      ? [
          grafta && `Called grafta ${grafta === 1 ? "once" : `${grafta} times`}`,
          others && `used ${plural(others, "other tool")}`,
        ]
          .filter(Boolean)
          .join(", ")
      : grafta
        ? `Used grafta${calls.length > 1 ? ` and ${plural(calls.length - 1, "more step")}` : ""}`
        : `Used ${plural(others, "other tool")}`;

  return (
    <div>
      <button
        type="button"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => {
          setOpen((o) => !o);
        }}
        className={cn(
          "group flex w-full cursor-pointer items-center gap-2 text-left",
          terminal
            ? "font-mono text-[13px] text-paper hover:text-bud"
            : "text-[15px] text-[#6b6a66] hover:text-[#141413]",
        )}
      >
        {terminal ? (
          <span aria-hidden className="text-sap">
            ⏺
          </span>
        ) : null}
        <span className={cn("min-w-0 truncate", running && !terminal && "text-[#141413]")}>{summary}</span>
        {running ? (
          <span
            aria-label="working"
            className="size-3 flex-none animate-spin rounded-full border-2 border-sap border-t-transparent"
          />
        ) : (
          <svg
            aria-hidden
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="currentColor"
            className={cn("flex-none transition-transform", open && "rotate-90")}
          >
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
        {terminal && !running ? <span className="text-[#9da3ab]">(click to expand)</span> : null}
        {!terminal && pictures.length ? (
          <span aria-hidden className="ml-auto flex flex-none items-center gap-1.5">
            {pictures.length > 3 ? <span className="mr-1 text-[13px]">+{pictures.length - 3}</span> : null}
            {pictures.slice(-3).map((p) => (
              <img
                fetchPriority="low"
                decoding="async"
                key={p.src}
                src={`${import.meta.env.BASE_URL}${p.src}`}
                alt=""
                loading="lazy"
                className="size-7 rounded-md border border-black/10 bg-white object-cover shadow-sm"
              />
            ))}
          </span>
        ) : null}
      </button>
      <div
        id={id}
        role="region"
        aria-label="Steps"
        hidden={!open}
        className={cn("mt-2 grid gap-2", terminal ? "border-l border-white/15 pl-4" : "")}
      >
        {open &&
          rows.map((r) => (
            <ToolRow
              key={r.index}
              tone={tone}
              label={r.turn.label}
              calls={r.turn.calls}
              other={r.turn.other}
              running={r.running}
            />
          ))}
      </div>
      {after ? <div className={terminal ? "mt-3 font-sans" : "mt-4"}>{after}</div> : null}
    </div>
  );
}
