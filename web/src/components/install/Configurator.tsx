import type { ReactNode } from "react";
import { clients } from "~/content/clients";
import { useTabs } from "../useTabs";
import { CopyButton } from "./CopyButton";

interface Props {
  /** Each client's snippet as highlighted HTML, made at build time (no highlighter in the browser). */
  highlighted: Record<string, string>;
  value: string;
  onValueChange: (id: string) => void;
  /** A client's mark or glyph, shown before its name. */
  glyphs?: Record<string, ReactNode> | undefined;
  /** After the tabs: a client that is not here yet. */
  after?: ReactNode | undefined;
}

/** Pick a client; get its command or configuration, the file it goes in, and one button to copy it. */
export function Configurator({ highlighted, value, onValueChange, glyphs, after }: Props) {
  const tabs = useTabs(
    clients.map((client) => client.id),
    value,
    onValueChange,
  );
  return (
    <div className="mx-auto max-w-[780px] text-left">
      <div {...tabs.list("Install grafta in")} className="mb-3 flex flex-wrap justify-center gap-1.5">
        {clients.map((client, i) => (
          <button
            key={client.id}
            {...tabs.tab(client.id, i)}
            className="inline-flex cursor-pointer items-center gap-2 rounded-full border border-line px-3.5 py-1.5 text-[14px] font-medium text-muted hover:text-text data-[state=active]:border-text data-[state=active]:bg-text data-[state=active]:text-bg"
          >
            {glyphs?.[client.id]}
            {client.name}
          </button>
        ))}
        {after}
      </div>
      {clients.map((client) => (
        // Every panel is drawn, the others hidden, so each configuration is in the page's HTML.
        <div key={client.id} {...tabs.panel(client.id)}>
          <div className="overflow-hidden rounded-2xl border border-line bg-surface">
            {client.file && (
              <div className="border-b border-line px-5 py-2.5 font-mono text-[12px] text-muted">
                {client.file}
              </div>
            )}
            <div className="flex items-center gap-3 py-3 pr-3 pl-5">
              <div
                className="min-w-0 flex-1 overflow-x-auto font-mono text-[14px] leading-relaxed whitespace-nowrap [&_pre]:m-0"
                // Shiki's output for a snippet in content/clients.ts, made at build time.
                dangerouslySetInnerHTML={{ __html: highlighted[client.id] ?? escape(client.snippet) }}
              />
              {client.kind === "download" ? (
                <a
                  href={client.snippet}
                  className="flex-none rounded-lg bg-button px-3 py-1.5 text-[14px] font-semibold text-on-button no-underline"
                >
                  Download
                </a>
              ) : (
                <CopyButton text={client.snippet} tone="primary" />
              )}
            </div>
          </div>
          <p className="mt-3 text-center text-[14px] text-muted">{client.note}</p>
        </div>
      ))}
    </div>
  );
}

function escape(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
