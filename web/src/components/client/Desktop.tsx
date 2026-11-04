import type { ReactNode } from "react";
import { cn } from "~/lib/cn";
import { Reply, type LogRef, type View } from "./view";
import { blocks, Work } from "./Work";

/**
 * Claude Desktop, redrawn from the measures of MCP Apps for Claude (Community),
 * CC BY 4.0: the sidebar, the conversation's title, the
 * person's bubble, the replies, the "Used grafta" rows and the input with its
 * model. No mark of the product is drawn; the type is the site's.
 */
export function Desktop({
  view,
  logRef,
  title,
  model,
  inline,
}: {
  view: View;
  logRef: LogRef;
  title: string;
  model: string;
  inline?: ReactNode;
}) {
  return (
    <div className="flex h-full min-h-0 overflow-hidden rounded-2xl border-[0.5px] border-[#1e1e1d]/30 bg-[#f9f9f7] text-[#141413] shadow-[0_12px_32px_rgb(0_0_0/0.06)]">
      <div
        aria-hidden
        className="hidden w-[52px] flex-none flex-col items-center gap-6 border-r-[0.5px] border-[#1f1e1d]/30 bg-[#faf9f5] pt-5 sm:flex"
      >
        {SIDEBAR.map((d, i) => (
          <svg
            key={i}
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#5e5d59"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className={i === 1 ? "mt-4" : undefined}
          >
            {d.map((p) => (
              <path key={p} d={p} />
            ))}
          </svg>
        ))}
      </div>
      <div className="flex min-w-0 flex-1 flex-col">
        <p className="flex-none px-5 pt-5 pb-2 text-[15px]">{title}</p>
        <div
          ref={logRef}
          role="log"
          aria-label="Conversation"
          className="mx-auto flex w-full max-w-[560px] min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pt-3 pb-6 sm:px-6"
        >
          <ol className="flex flex-col gap-4">
            {blocks(view.turns, view.at, view.finished).map((block) => {
              if (block.kind === "work") {
                const key = block.rows[0]?.index ?? 0;
                const hasLast = block.rows.some((r) => r.index === view.lastTools);
                return (
                  <li key={key}>
                    <Work rows={block.rows} tone="desktop" after={inline && hasLast ? inline : undefined} />
                  </li>
                );
              }
              const { turn, index: i } = block;
              if (turn.kind === "user") {
                if (i === view.at && !view.finished) return null;
                return (
                  <li key={i} className="flex justify-end">
                    <p className="max-w-[min(420px,90%)] rounded-xl bg-[#f0efec] py-3 pr-4 pl-3 text-[15px] leading-[22px]">
                      {turn.text}
                    </p>
                  </li>
                );
              }
              return (
                <li key={i} className="text-[15px] leading-6">
                  <Reply text={i === view.at ? turn.text.slice(0, view.chars) : turn.text} />
                </li>
              );
            })}
            {view.working ? (
              <li aria-hidden>
                <Spark />
              </li>
            ) : null}
          </ol>
        </div>
        <div className="flex-none px-3 pb-3 sm:px-6 sm:pb-4">
          <div className="mx-auto max-w-[600px] rounded-2xl border-[0.5px] border-[#1e1e1d]/30 bg-white p-3.5 shadow-[0_2px_8px_rgb(0_0_0/0.08)]">
            <p
              className={cn(
                "min-h-[22px] text-[15px] leading-[22px]",
                view.typing ? "text-[#141413]" : "text-[#6b6a66]",
              )}
            >
              {view.typing ?? "Reply to Claude"}
              {view.typing !== undefined ? (
                <span className="ml-px inline-block h-4 w-px animate-pulse bg-[#141413] align-middle" />
              ) : null}
            </p>
            <div className="mt-3 flex items-center justify-end gap-2 text-[13px]">
              <span>{model}</span>
              <span aria-hidden className="grid size-8 place-items-center rounded-lg bg-[#c6613f] text-white">
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="m5 12 7-7 7 7" />
                  <path d="M12 19V5" />
                </svg>
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/** A generic asterisk while the model works, in the kit's send colour: not the product's mark. */
function Spark() {
  return (
    <svg
      width="22"
      height="22"
      viewBox="0 0 24 24"
      className="animate-[spin_3s_linear_infinite] text-[#c6613f]"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
    >
      <path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" />
    </svg>
  );
}

const SIDEBAR = [
  ["M3 3h18v18H3z", "M9 3v18"],
  ["M12 5v14", "M5 12h14"],
  ["m21 21-4.3-4.3", "M11 3a8 8 0 1 0 0 16 8 8 0 0 0 0-16z"],
  ["M4 4h16v16H4z", "M4 9h16"],
];
