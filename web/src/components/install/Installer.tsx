import { clients } from "~/content/clients";
import type { Client } from "~/content/types";
import { choose, useChoice } from "./choice";
import { ClientMark } from "./ClientMark";
import { CopyButton } from "./CopyButton";

const GROUPS: { id: Client["group"]; name: string }[] = [
  { id: "app", name: "In your AI app" },
  { id: "program", name: "Or as a program, for any app" },
];

/** The chosen way in, for the steps after this one (the sample's note). */
export function useClient(): Client | undefined {
  const chosen = useChoice();
  return clients.find((item) => item.id === chosen) ?? clients[0];
}

/**
 * Adding grafta to an AI app: the apps as one strip of tiles, each with its mark,
 * its name and its step in a few words; the program's ways in a quieter line
 * under it; then the chosen one's step with its one button.
 */
export function Installer() {
  const client = useClient();
  if (!client) return null;
  return (
    <div className="min-w-0">
      {GROUPS.map((group) =>
        group.id === "app" ? (
          <fieldset key={group.id} className="min-w-0">
            <legend className="sr-only">{group.name}</legend>
            <div className="grid grid-cols-2 gap-px overflow-hidden rounded-xl bg-line ring-1 ring-line sm:grid-cols-4 lg:grid-cols-7">
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
                    className="group flex min-w-0 cursor-pointer flex-col items-start gap-3 bg-surface px-4 py-4 text-left last:col-span-2 lg:last:col-span-1 transition-colors hover:bg-raised aria-pressed:bg-raised aria-pressed:shadow-[inset_0_-2px_0_var(--accent)]"
                  >
                    <ClientMark id={item.id} size={20} />
                    <span className="min-w-0">
                      <span className="block text-[14.5px] leading-snug font-[510] text-text">
                        {item.name}
                      </span>
                      <span aria-hidden="true" className="block truncate font-mono text-[11.5px] text-faint">
                        {item.hint}
                      </span>
                    </span>
                  </button>
                ))}
            </div>
          </fieldset>
        ) : (
          <fieldset key={group.id} className="mt-4 min-w-0 sm:flex sm:items-center sm:gap-3">
            <legend className="sr-only">{group.name}</legend>
            <span aria-hidden="true" className="mb-1.5 block text-[13px] text-faint sm:mb-0 sm:flex-none">
              Or as a program:
            </span>
            <div className="flex flex-wrap gap-1">
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
                    className="cursor-pointer rounded-md px-2 py-1 text-[13px] text-muted underline decoration-line underline-offset-4 hover:text-text aria-pressed:bg-surface aria-pressed:text-text aria-pressed:no-underline aria-pressed:ring-1 aria-pressed:ring-text aria-pressed:ring-inset"
                  >
                    {item.name}
                  </button>
                ))}
            </div>
          </fieldset>
        ),
      )}

      <section
        aria-label={`Install it ${client.group === "app" ? "in" : "with"} ${client.name}`}
        className="mt-6"
      >
        <Snippet client={client} />
        <p className="mt-3 max-w-[640px] text-[14px] leading-relaxed text-muted">{client.note}</p>
      </section>
    </div>
  );
}

/** One thing to take: a file, a line or a request, with its one button. */
export function Line({ className, children }: { className?: string; children: React.ReactNode }) {
  return (
    <div
      className={`flex items-center justify-between gap-3 rounded-[10px] bg-surface px-4 py-3 ring-1 ring-line ${className ?? ""}`}
    >
      {children}
    </div>
  );
}

function Snippet({ client }: { client: Client }) {
  if (client.kind === "download") {
    const file = client.snippet.split("/").at(-1) ?? client.snippet;
    return (
      <Line>
        <span className="min-w-0 truncate font-mono text-[14px]">{file}</span>
        <a
          href={client.snippet}
          className="flex-none rounded-lg bg-button px-3.5 py-1.5 text-[14px] font-semibold text-on-button no-underline hover:opacity-90"
        >
          Download
        </a>
      </Line>
    );
  }
  return (
    <div className="rounded-[10px] bg-surface ring-1 ring-line">
      {client.file ? (
        <p className="border-b border-line px-4 py-2 font-mono text-[12px] text-muted">{client.file}</p>
      ) : null}
      <div className="flex items-start justify-between gap-3 px-4 py-3.5">
        <code className="block min-w-0 overflow-x-auto font-mono text-[14px] leading-6 whitespace-pre">
          <span aria-hidden="true" className="text-faint select-none">
            {client.kind === "command" && !client.snippet.startsWith("/") ? "$ " : ""}
          </span>
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
