import { Rich } from "../Rich";
import { summary } from "./summary";
import type { Tool } from "./types";

/**
 * A tool as a disclosure: closed, its name, the first of its own words and how
 * many arguments it takes; open, all its words and its arguments, required first,
 * as a list that wraps on any screen. Opens without scripts.
 */
export function ToolRow({
  tool,
  open,
  onToggle,
}: {
  tool: Tool;
  open: boolean;
  onToggle: (name: string, open: boolean) => void;
}) {
  const count = tool.arguments.length;
  return (
    <details
      id={tool.name}
      open={open}
      onToggle={(event) => {
        onToggle(tool.name, event.currentTarget.open);
      }}
      className="tool group scroll-mt-[9.5rem] border-b border-line last:border-0 [overflow-wrap:anywhere]"
    >
      <summary className="grid cursor-pointer list-none grid-cols-[1rem_minmax(0,1fr)_auto] items-baseline gap-x-3 gap-y-1 px-4 py-3.5 hover:bg-bg sm:grid-cols-[1rem_13rem_minmax(0,1fr)_auto] sm:px-5 [&::-webkit-details-marker]:hidden">
        <span aria-hidden="true" className="text-[12px] text-muted transition-transform group-open:rotate-90">
          ▸
        </span>
        <span className="font-mono text-[15px] font-semibold text-text">{tool.name}</span>
        <span className="col-start-2 row-start-2 text-[14.5px] text-muted sm:col-start-3 sm:row-start-1 sm:truncate">
          <Rich text={summary(tool.description)} />
        </span>
        <span className="col-start-3 row-start-1 text-right text-[13px] whitespace-nowrap text-muted sm:col-start-4">
          {count === 0 ? "no arguments" : count === 1 ? "1 argument" : `${count} arguments`}
        </span>
      </summary>
      <div className="px-4 pb-6 sm:pr-5 sm:pl-12">
        <p className="max-w-[72ch] text-[15px] leading-relaxed">
          <Rich text={tool.description} />
        </p>
        {count > 0 ? (
          <dl className="mt-5 max-w-[72ch] divide-y divide-line border-y border-line">
            {[...tool.arguments]
              .sort((a, b) => Number(b.required) - Number(a.required))
              .map((arg) => (
                <div key={arg.name} className="py-3">
                  <dt className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
                    <code className="text-[14px] font-semibold">{arg.name}</code>
                    <span className="font-mono text-[12.5px] text-muted">{arg.type}</span>
                    {arg.required ? (
                      <span className="rounded-full border border-line px-2 text-[12px] text-accent-text">
                        required
                      </span>
                    ) : null}
                  </dt>
                  <dd className="mt-1 text-[14.5px] leading-relaxed text-muted">
                    <Rich text={arg.description} />
                  </dd>
                </div>
              ))}
          </dl>
        ) : null}
      </div>
    </details>
  );
}
