import type { ReactNode } from "react";
import { Reply, type LogRef, type View } from "./view";
import { blocks, Work } from "./Work";

/** Claude Code, in a terminal: the same turns as text, as the recording has them. */
export function Terminal({ view, logRef, inline }: { view: View; logRef: LogRef; inline?: ReactNode }) {
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-[14px] bg-ink text-paper shadow-[0_12px_32px_rgb(0_0_0/0.12)]">
      <div aria-hidden className="flex flex-none items-center gap-2 px-4 pt-3.5 pb-2">
        <span className="size-3 rounded-full bg-[#ff5f57]" />
        <span className="size-3 rounded-full bg-[#febc2e]" />
        <span className="size-3 rounded-full bg-[#28c840]" />
        <span className="flex-1 text-center font-mono text-[12px] text-[#9da3ab]">~/designs — claude</span>
      </div>
      <div
        ref={logRef}
        role="log"
        aria-label="Conversation"
        className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-6 pt-2 pb-4 font-mono text-[13px] leading-5"
      >
        <ol className="flex flex-col gap-3">
          {blocks(view.turns, view.at, view.finished).map((block) => {
            if (block.kind === "work") {
              const key = block.rows[0]?.index ?? 0;
              const hasLast = block.rows.some((r) => r.index === view.lastTools);
              return (
                <li key={key}>
                  <Work rows={block.rows} tone="terminal" after={inline && hasLast ? inline : undefined} />
                </li>
              );
            }
            const { turn, index: i } = block;
            if (turn.kind === "user") {
              if (i === view.at && !view.finished) return null;
              return (
                <li key={i} className="flex gap-2">
                  <span aria-hidden className="text-[#9da3ab]">
                    &gt;
                  </span>
                  <span>{turn.text}</span>
                </li>
              );
            }
            return (
              <li key={i} className="flex gap-2">
                <span aria-hidden>⏺</span>
                <span className="min-w-0">
                  <Reply text={i === view.at ? turn.text.slice(0, view.chars) : turn.text} />
                </span>
              </li>
            );
          })}
          {view.working ? (
            <li aria-hidden className="text-bud">
              * Working…
            </li>
          ) : null}
        </ol>
      </div>
      <div className="flex-none px-4 pb-4">
        <p className="rounded-lg border border-paper/25 px-3 py-2 text-[13px] text-paper">
          <span className="text-[#9da3ab]">&gt; </span>
          {view.typing}
          {view.typing !== undefined ? (
            <span className="ml-px inline-block h-4 w-2 animate-pulse bg-paper align-middle" />
          ) : null}
        </p>
      </div>
    </div>
  );
}
