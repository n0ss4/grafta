import { useId, useState } from "react";
import { cn } from "~/lib/cn";
import type { Step } from "./types";

/** A call's arguments without the handle, which says nothing to a reader. */
export function shownArgs(step: Step): string {
  const args = Object.fromEntries(Object.entries(step.args).filter(([key]) => key !== "handle"));
  return Object.keys(args).length ? JSON.stringify(args) : "{ }";
}

/** The result's excerpt, indented when it parses. */
export function pretty(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

/**
 * A run of calls the model made, as one row: what it did in a few words,
 * opening to each call as it was recorded. The client's own
 * tools read "Used another tool" and open the same way.
 */
export function ToolRow({
  label,
  calls,
  other = false,
  running = false,
  tone = "desktop",
}: {
  label: string;
  calls: Step[];
  other?: boolean | undefined;
  /** The row's last call has not answered yet, while playing. */
  running?: boolean | undefined;
  tone?: "desktop" | "terminal" | undefined;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const terminal = tone === "terminal";
  return (
    <div
      className={cn(
        "overflow-hidden",
        terminal ? "font-mono text-[13px]" : "rounded-[10px] border-[0.5px] border-[#1e1e1d]/30 bg-white",
      )}
    >
      <button
        type="button"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => {
          setOpen((o) => !o);
        }}
        className={cn(
          "flex w-full cursor-pointer items-center gap-2.5 text-left",
          terminal ? "py-0.5 text-paper hover:text-bud" : "px-3 py-2.5 hover:bg-[#f9f9f7]",
        )}
      >
        {terminal ? (
          <span aria-hidden className={other ? "text-[#9da3ab]" : "text-sap"}>
            ⏺
          </span>
        ) : (
          <Layers className={other ? "text-[#6b6a66]" : "text-deep-sap"} />
        )}
        {!terminal && !other && <span className="text-[13px] font-semibold text-deep-sap">grafta</span>}
        <span className={cn("min-w-0 flex-1 truncate", terminal ? "" : "text-[14px] text-[#141413]")}>
          {label}
        </span>
        <span className={cn("flex-none text-[12px]", terminal ? "text-[#9da3ab]" : "text-[#6b6a66]")}>
          {calls.length} call{calls.length === 1 ? "" : "s"}
        </span>
        {running ? (
          <span
            aria-label="working"
            className="size-3 flex-none animate-spin rounded-full border-2 border-sap border-t-transparent"
          />
        ) : null}
        <Chevron open={open} />
      </button>
      <div
        id={id}
        role="region"
        aria-label="Calls"
        hidden={!open}
        className={cn(
          "grid gap-3",
          terminal
            ? "border-l border-white/15 py-1 pl-4"
            : "border-t-[0.5px] border-[#1e1e1d]/15 bg-[#fbfaf6] p-3",
        )}
      >
        {/* Drawn when opened: closed, the page carries none of it. */}
        {open &&
          calls.map((call, i) => (
            <div key={i} className="min-w-0">
              <p
                className={cn(
                  "font-mono text-[12px] font-bold break-all",
                  terminal ? "text-paper" : "text-ink",
                )}
              >
                {call.tool} <span className="font-normal">{shownArgs(call)}</span>
              </p>
              {call.result ? (
                <pre
                  className={cn(
                    "mt-1 font-mono text-[11px] leading-[17px] break-all whitespace-pre-wrap",
                    terminal ? "text-[#9da3ab]" : "text-[#4f535a]",
                  )}
                >
                  {pretty(call.result)}
                  {call.omitted ? `\n… ${call.omitted}` : ""}
                </pre>
              ) : null}
              {call.picture ? (
                <img
                  src={`${import.meta.env.BASE_URL}${call.picture.src}`}
                  width={call.picture.width}
                  height={call.picture.height}
                  alt={call.picture.alt}
                  loading="lazy"
                  className="mt-2 h-auto max-w-full rounded-md border border-black/10"
                />
              ) : null}
            </div>
          ))}
      </div>
    </div>
  );
}

function Layers({ className }: { className?: string }) {
  return (
    <svg
      aria-hidden
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("flex-none", className)}
    >
      <path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z" />
      <path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12" />
      <path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17" />
    </svg>
  );
}

function Chevron({ open }: { open: boolean }) {
  return (
    <svg
      aria-hidden
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("flex-none text-[#6b6a66] transition-transform", open && "rotate-90")}
    >
      <path d="m9 18 6-6-6-6" />
    </svg>
  );
}
