import { clients, firstRequest, sample } from "~/content/clients";
import type { Client } from "~/content/types";
import { cn } from "~/lib/cn";
import { href } from "~/lib/seo";
import { choose, useChoice } from "./choice";
import { ClientMark } from "./ClientMark";
import { CopyButton } from "./CopyButton";

const GROUPS: { id: Client["group"]; name: string }[] = [
  { id: "app", name: "In your AI app" },
  { id: "program", name: "Or as a program, for any app" },
];

/**
 * Getting grafta: the ways in, in two groups, the chosen one's step with its one
 * button, and the sample with the first request to try. The same in the install
 * section and in the dialog; the choice is shared (choice.ts).
 */
export function Installer({ compact = false }: { compact?: boolean }) {
  const chosen = useChoice();
  const client = clients.find((item) => item.id === chosen) ?? clients[0];
  if (!client) return null;
  return (
    <div>
      {GROUPS.map((group) => (
        <fieldset key={group.id} className="mt-5 first:mt-0">
          <legend className="text-[14px] text-muted">{group.name}</legend>
          <div className="mt-2 flex flex-wrap gap-2">
            {clients
              .filter((item) => item.group === group.id)
              .map((item) => (
                <button
                  type="button"
                  key={item.id}
                  onClick={() => {
                    choose(item.id);
                  }}
                  aria-pressed={item.id === client.id}
                  className="flex cursor-pointer items-center gap-2 rounded-full border border-line bg-surface px-3.5 py-2 text-[14px] text-text transition-colors hover:border-text aria-pressed:border-accent aria-pressed:font-medium aria-pressed:shadow-[0_0_0_1px_var(--accent)]"
                >
                  <ClientMark id={item.id} size={16} />
                  {item.name}
                </button>
              ))}
          </div>
        </fieldset>
      ))}

      <div className={cn("mt-8 grid gap-4", !compact && "lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)]")}>
        <Step number={1} title={`Install it ${client.group === "app" ? "in" : "with"} ${client.name}`}>
          <Snippet client={client} />
          <p className="mt-3 text-[13px] leading-relaxed text-muted">{client.note}</p>
        </Step>
        <Step number={2} title="Try it on a Figma file">
          <div className="flex items-center justify-between gap-3 rounded-[10px] border border-line bg-bg px-3.5 py-3">
            <span className="min-w-0 truncate font-mono text-[13px]">{sample.file}, a sample</span>
            <a
              href={href(sample.href)}
              download
              className="flex-none rounded-lg border border-line px-3 py-1.5 text-[14px] font-semibold text-text no-underline hover:border-text"
            >
              Download
            </a>
          </div>
          <p className="mt-3 text-[13px] text-muted">Then ask your AI:</p>
          <div className="mt-1.5 flex items-center justify-between gap-3 rounded-[10px] border border-line bg-bg px-3.5 py-3">
            <span className="min-w-0 text-[15px]">“{firstRequest}”</span>
            <CopyButton text={firstRequest} label="Copy" />
          </div>
          {client.tryNote ? (
            <p className="mt-3 text-[13px] leading-relaxed text-muted">{client.tryNote}</p>
          ) : null}
        </Step>
      </div>
    </div>
  );
}

function Step({ number, title, children }: { number: number; title: string; children: React.ReactNode }) {
  return (
    <section aria-label={title} className="min-w-0 rounded-2xl border border-line bg-surface p-5">
      <h3 className="flex items-baseline gap-3 text-[17px] font-medium">
        <span className="font-mono text-[13px] text-accent-text">{number}</span>
        {title}
      </h3>
      <div className="mt-4">{children}</div>
    </section>
  );
}

function Snippet({ client }: { client: Client }) {
  if (client.kind === "download") {
    const file = client.snippet.split("/").at(-1) ?? client.snippet;
    return (
      <div className="flex items-center justify-between gap-3 rounded-[10px] border border-line bg-bg px-3.5 py-3">
        <span className="min-w-0 truncate font-mono text-[13px]">{file}</span>
        <a
          href={client.snippet}
          className="flex-none rounded-lg bg-button px-3.5 py-1.5 text-[14px] font-semibold text-on-button no-underline hover:opacity-90"
        >
          Download
        </a>
      </div>
    );
  }
  return (
    <div className="rounded-[10px] border border-line bg-bg">
      {client.file ? (
        <p className="border-b border-line px-3.5 py-2 font-mono text-[12px] text-muted">{client.file}</p>
      ) : null}
      <div className="flex items-start justify-between gap-3 px-3.5 py-3">
        <code className="block min-w-0 overflow-x-auto font-mono text-[13px] leading-5 whitespace-pre">
          {client.snippet}
        </code>
        <CopyButton
          text={client.snippet}
          label={client.kind === "json" ? "Copy configuration" : "Copy line"}
          tone="primary"
        />
      </div>
    </div>
  );
}
