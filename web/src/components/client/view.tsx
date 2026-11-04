import type { ReactNode, RefObject } from "react";
import type { Turn } from "./types";

/** What a client draws at a moment of the playback; Client computes it. */
export interface View {
  turns: Turn[];
  /** The turn being played; turns after it are not shown. */
  at: number;
  /** Characters of the current reply shown so far. */
  chars: number;
  /** The current turn has finished (its text shown whole, its calls answered). */
  finished: boolean;
  /** The prompt as it is being typed into the input; undefined once sent. */
  typing: string | undefined;
  /** The model is still working: the spark shows. */
  working: boolean;
  /** The last grafta row shown, where a phone draws the design inline. */
  lastTools: number;
}

/** The conversation's list, which Client scrolls as turns arrive. */
export type LogRef = RefObject<HTMLDivElement | null>;

/** A picture the importer pointed at the site: `![alt](showcase/…)`. */
const BULLET = /^\s*(?:[-*]|\d+\.)\s/;
const PICTURE = /^!\[([^\]]*)\]\((showcase\/[^)\s]+)\)$/;

/** `code` and **bold** in a line, as the clients render them. */
function inline(text: string, key: string): ReactNode[] {
  return text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g).map((part, i) => {
    if (part.startsWith("`") && part.endsWith("`") && part.length > 1)
      return (
        <code key={`${key}-${i}`} className="rounded bg-black/[0.06] px-1 font-mono text-[0.9em]">
          {part.slice(1, -1)}
        </code>
      );
    if (part.startsWith("**") && part.endsWith("**") && part.length > 3)
      return <strong key={`${key}-${i}`}>{part.slice(2, -2)}</strong>;
    return part;
  });
}

/**
 * A reply as the model wrote it, with the Markdown the clients render:
 * paragraphs, bulleted lines, tables, inline code and bold. Nothing is
 * reworded; anything else shows as written.
 */
export function Reply({ text }: { text: string }) {
  const blocks = text.split(/\n{2,}/);
  return (
    <>
      {blocks.map((block, b) => {
        const lines = block.split("\n");
        if (lines.every((l) => l.trim().startsWith("|"))) {
          const rows = lines
            .filter((l) => !/^\s*\|[\s:|-]+\|\s*$/.test(l))
            .map((l) =>
              l
                .trim()
                .replace(/^\||\|$/g, "")
                .split("|")
                .map((c) => c.trim()),
            );
          const [head, ...body] = rows;
          return (
            <table key={b} className="my-2 w-full border-collapse text-left text-[0.9em]">
              {head ? (
                <thead>
                  <tr>
                    {head.map((c, i) => (
                      <th key={i} className="border-b border-current/20 py-1 pr-3 font-semibold">
                        {inline(c, `h${b}${i}`)}
                      </th>
                    ))}
                  </tr>
                </thead>
              ) : null}
              <tbody>
                {body.map((r, j) => (
                  <tr key={j}>
                    {r.map((c, i) => (
                      <td key={i} className="border-b border-current/10 py-1 pr-3">
                        {inline(c, `c${b}${j}${i}`)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          );
        }
        // Lines in order: a picture alone, a run of bulleted lines as a list, the rest as text.
        const parts: ReactNode[] = [];
        let items: string[] = [];
        const flush = (i: number) => {
          if (!items.length) return;
          parts.push(
            <ul key={`u${i}`} className="my-1 list-disc pl-5">
              {items.map((item, j) => (
                <li key={j}>{inline(item, `u${b}${i}${j}`)}</li>
              ))}
            </ul>,
          );
          items = [];
        };
        lines.forEach((l, i) => {
          const bullet = BULLET.exec(l);
          if (bullet) {
            items.push(l.slice(bullet[0].length));
            return;
          }
          flush(i);
          const picture = PICTURE.exec(l.trim());
          parts.push(
            picture ? (
              <img
                key={i}
                src={`${import.meta.env.BASE_URL}${picture[2]}`}
                alt={picture[1] ?? ""}
                loading="lazy"
                className="my-2 block h-auto max-w-[280px] rounded-md border border-black/10"
              />
            ) : (
              <p key={i}>{inline(l.replace(/^#+\s/, ""), `p${b}${i}`)}</p>
            ),
          );
        });
        flush(lines.length);
        return (
          <div key={b} className={b ? "mt-3" : undefined}>
            {parts}
          </div>
        );
      })}
    </>
  );
}
