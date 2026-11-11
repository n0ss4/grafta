import { useEffect, useState } from "react";
import { cn } from "~/lib/cn";
import type { Summary, Tokens as TokensResult, Variables, VariableValue } from "./protocol";
import type { Reader, ReaderError } from "./reader";
import { save, zip } from "./save";
import { archiveName } from "./state";

/** Variables asked for at once, until the file has given them all. */
const PAGE_SIZE = 200;

type Variable = Variables["results"][number];

/** Where each export's files are written, in the reader's memory, before they are taken. */
let exports = 0;

/** The file's variables by collection, and the same as W3C Design Tokens to download. */
export function Tokens({ reader, summary }: { reader: Reader; summary: Summary }) {
  const [collections, setCollections] = useState<Variables["collections"]>([]);
  const [variables, setVariables] = useState<Variable[] | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const [files, setFiles] = useState<{ name: string; bytes: Uint8Array }[]>([]);
  const [said, setSaid] = useState<TokensResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const run = { live: true };
    const alive = () => run.live;
    (async () => {
      const all: Variable[] = [];
      let cursor: string | undefined;
      let found: Variables;
      do {
        found = await reader.call<Variables>("list_variables", {
          handle: summary.handle,
          limit: PAGE_SIZE,
          ...(cursor ? { cursor } : {}),
        });
        all.push(...found.results);
        cursor = found.next_cursor;
      } while (cursor && alive());
      if (!alive()) return;
      setCollections(found.collections);
      setChosen(found.collections.find((c) => c.variables > 0)?.id ?? null);
      setVariables(all);

      // The export, once: what it could not write is shown before anyone downloads.
      exports += 1;
      const out = await reader.call<TokensResult>("export_tokens", {
        handle: summary.handle,
        dir: `/tokens/${exports}`,
      });
      const taken = [];
      for (const file of out.files) {
        taken.push({
          name: file.path.split(/[/\\]/).pop() ?? file.path,
          bytes: await reader.take(file.path),
        });
      }
      if (!alive()) return;
      setSaid(out);
      setFiles(taken);
    })().catch((caught: unknown) => {
      if (alive()) setError((caught as ReaderError).message);
    });
    return () => {
      run.live = false;
    };
  }, [reader, summary.handle]);

  if (error)
    return (
      <p role="alert" className="text-limit">
        {error}
      </p>
    );
  if (!variables) return <p className="text-muted">Reading the variables…</p>;
  if (variables.length === 0) {
    return <p className="text-muted">This file has no variables, so there are no tokens to take.</p>;
  }

  const collection = collections.find((c) => c.id === chosen);
  const shown = variables.filter((v) => v.collection === collection?.name);

  async function download() {
    const [only] = files;
    if (files.length === 1 && only) {
      save(only.name, new Blob([only.bytes as Uint8Array<ArrayBuffer>], { type: "application/json" }));
      return;
    }
    save(archiveName(summary.file, "tokens"), await zip(files));
  }

  return (
    <div className="grid gap-8 lg:grid-cols-[13rem_1fr]">
      <nav aria-label="Collections" className="lg:sticky lg:top-20 lg:self-start">
        <h3 className="micro mb-2 text-muted">Collections</h3>
        <ul className="flex gap-1 overflow-x-auto lg:flex-col">
          {collections
            .filter((c) => c.variables > 0)
            .map((c) => (
              <li key={c.id}>
                <button
                  type="button"
                  aria-current={c.id === chosen ? "true" : undefined}
                  onClick={() => {
                    setChosen(c.id);
                  }}
                  className={cn(
                    "w-full cursor-pointer rounded-md px-2.5 py-1.5 text-left text-[14px] whitespace-nowrap",
                    c.id === chosen ? "bg-soft font-semibold text-accent-text" : "text-text hover:bg-sunk",
                  )}
                >
                  {c.name} <span className="font-normal text-muted">{c.variables}</span>
                </button>
              </li>
            ))}
        </ul>
      </nav>

      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            disabled={files.length === 0}
            onClick={() => void download()}
            className="cursor-pointer rounded-lg bg-button px-3 py-1.5 text-[14px] font-semibold text-on-button hover:opacity-90 disabled:opacity-60"
          >
            {files.length === 0
              ? "Writing the tokens…"
              : files.length === 1
                ? "Download the tokens"
                : `Download ${files.length} token files`}
          </button>
          <p className="text-[14px] text-muted">
            W3C Design Tokens, one file per collection and mode, the way Figma imports them.
          </p>
        </div>
        {said && <Said said={said} />}

        {collection && (
          <div className="mt-6 overflow-x-auto rounded-lg border border-line bg-surface">
            <table className="w-full text-left text-[14px]">
              <caption className="sr-only">{collection.name}: its variables, by mode</caption>
              <thead>
                <tr className="border-b border-line">
                  <th scope="col" className="px-4 py-2.5 font-semibold">
                    Variable
                  </th>
                  {collection.modes.map((mode) => (
                    <th key={mode} scope="col" className="px-4 py-2.5 font-semibold">
                      {mode}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {shown.map((v) => (
                  <tr key={v.id} className="border-b border-hair last:border-0">
                    <th scope="row" className="px-4 py-2 font-normal break-all">
                      {v.name}
                    </th>
                    {collection.modes.map((mode) => (
                      <td key={mode} className="px-4 py-2">
                        <Value value={v.values[mode]} />
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

/** A value as the file holds it: a colour as a swatch and its hex, an alias by name. */
function Value({ value }: { value: VariableValue | undefined }) {
  if (value === undefined) return <span className="text-muted">none</span>;
  if (typeof value === "object" && "alias" in value)
    return <span className="text-muted">→ {value.alias}</span>;
  const colour =
    typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value) ? { hex: value, opacity: 1 } : null;
  const translucent =
    typeof value === "object" && "color" in value ? { hex: value.color, opacity: value.opacity } : null;
  const swatch = colour ?? translucent;
  if (swatch) {
    return (
      <span className="inline-flex items-center gap-2 font-mono text-[13px]">
        <span
          aria-hidden="true"
          className="inline-block size-4 rounded-sm outline-1 outline-line"
          style={{ backgroundColor: swatch.hex, opacity: swatch.opacity }}
        />
        {swatch.hex}
        {swatch.opacity < 1 && <span className="text-muted">{Math.round(swatch.opacity * 100)}%</span>}
      </span>
    );
  }
  return (
    <span className="font-mono text-[13px]">
      {typeof value === "object" ? JSON.stringify(value) : String(value)}
    </span>
  );
}

/** What the export could not write as Figma has it, said before the download. */
function Said({ said }: { said: TokensResult }) {
  const count = said.unresolved.length + said.renamed.length + said.notes.length;
  if (count === 0) return null;
  return (
    <details className="mt-4 text-[14px]">
      <summary className="cursor-pointer text-muted">
        {[
          said.unresolved.length > 0 && `${said.unresolved.length} not written`,
          said.renamed.length > 0 && `${said.renamed.length} renamed to fit the format`,
          said.notes.length > 0 && (said.notes.length === 1 ? "a note" : `${said.notes.length} notes`),
        ]
          .filter(Boolean)
          .join(", ")}
      </summary>
      <div className="mt-2 space-y-3 text-muted">
        {said.unresolved.length > 0 && (
          <div>
            <p className="font-semibold text-text">
              Not written: aliases to variables this file does not hold
            </p>
            <ul className="mt-1 list-disc pl-5">
              {said.unresolved.map((u) => (
                <li key={`${u.variable}-${u.mode}`}>
                  {u.variable} ({u.mode})
                </li>
              ))}
            </ul>
          </div>
        )}
        {said.renamed.length > 0 && (
          <div>
            <p className="font-semibold text-text">Renamed</p>
            <ul className="mt-1 list-disc pl-5">
              {said.renamed.map((r) => (
                <li key={r.from}>
                  {r.from} → {r.to}
                </li>
              ))}
            </ul>
          </div>
        )}
        {said.notes.map((note) => (
          <p key={note}>{note}</p>
        ))}
      </div>
    </details>
  );
}
