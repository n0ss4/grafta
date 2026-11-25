// The self-test, a ladder: the calls recorded on order.fig by
// crates/toolkit/tests/selftest.rs, run here on a copy of order.fig open in
// Figma, each answer compared with the file's: its keys, and for a read of the
// sample's own layers, its values. What it made is removed after, and the
// table is given as it came out. In development builds only.
import recording from "./selftest.json";
import { REGISTRY } from "./tools";

interface Case {
  tool: string;
  args: Record<string, unknown>;
  live: string;
  keys?: string[];
  values?: unknown;
  image?: boolean;
}

export interface Row {
  call: number;
  tool: string;
  live: string;
  outcome: "same" | "differs" | "refused" | "not run";
  notes: string;
  ms: number;
  /** Every difference, when there are more than the table names. */
  all?: string[];
}

const NOT_COMPARED = new Set(Object.keys(recording.not_compared));

/** A node id, in either place: told apart by shape alone, as ids are not the same in a copy. */
const ID = /^I?-?\d+:-?\d+(;-?\d+:-?\d+)*$/;

/** `"$N.path"` replaced by that field of result N, as grafta's batch does. */
function substitute(value: unknown, results: unknown[]): unknown {
  if (typeof value === "string" && /^\$\d/.test(value)) {
    const [index, ...path] = value.slice(1).split(".");
    let at: unknown = results[Number(index)];
    if (at === undefined || at === null) throw new Error(`needs call ${index}'s result, which there is not`);
    for (const part of path) {
      at = (at as Record<string, unknown>)?.[part];
      if (at === undefined) throw new Error(`call ${index}'s result has no \`${part}\``);
    }
    return at;
  }
  if (Array.isArray(value)) return value.map((item) => substitute(item, results));
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, substitute(item, results)]));
  }
  return value;
}

function comparable(value: unknown): unknown {
  if (typeof value === "string" && ID.test(value)) return "<id>";
  if (Array.isArray(value)) return value.map(comparable);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !NOT_COMPARED.has(key))
        .map(([key, item]) => [key, comparable(item)]),
    );
  }
  return value;
}

/** How many differences a case names; the rest are counted. */
const SHOWN = 5;

/** Every place two values differ, by path, with both sides. */
function differences(file: unknown, figmaSide: unknown, at: string, out: string[]): void {
  if (file && figmaSide && typeof file === "object" && typeof figmaSide === "object") {
    if (Array.isArray(file) !== Array.isArray(figmaSide)) {
      out.push(`${at || "the result"}: a list in one, not in the other`);
      return;
    }
    const keys = new Set([...Object.keys(file), ...Object.keys(figmaSide)]);
    for (const key of keys) {
      differences((file as Record<string, unknown>)[key], (figmaSide as Record<string, unknown>)[key], at ? `${at}.${key}` : key, out);
    }
    return;
  }
  if (JSON.stringify(file) !== JSON.stringify(figmaSide)) {
    out.push(`${at || "the result"}: file ${JSON.stringify(file) ?? "nothing"}, Figma ${JSON.stringify(figmaSide) ?? "nothing"}`);
  }
}

function compare(expected: Case, answer: unknown): { outcome: Row["outcome"]; notes: string; all?: string[] } {
  const result = (answer ?? {}) as Record<string, unknown>;
  if (expected.image) {
    return "image" in result ? { outcome: "same", notes: "a picture, as on the file" } : { outcome: "differs", notes: "no picture" };
  }
  const keys = Object.keys(result).filter((key) => !NOT_COMPARED.has(key));
  const missing = (expected.keys ?? []).filter((key) => !keys.includes(key));
  const extra = keys.filter((key) => !(expected.keys ?? []).includes(key));
  const notes: string[] = [];
  if (missing.length) notes.push(`not in Figma's: ${missing.join(", ")}`);
  if (extra.length) notes.push(`only in Figma's: ${extra.join(", ")}`);
  if (expected.values !== undefined) {
    const found: string[] = [];
    differences(comparable(expected.values), comparable(result), "", found);
    notes.push(...found.slice(0, SHOWN));
    if (found.length > SHOWN) {
      notes.push(`and ${found.length - SHOWN} more, below`);
      return { outcome: "differs", notes: notes.join("; "), all: found };
    }
  }
  return notes.length ? { outcome: "differs", notes: notes.join("; ") } : { outcome: "same", notes: "" };
}

/** What the self-test made, removed: its page, its collection, its style. */
async function cleanUp(cases: Case[], results: unknown[]): Promise<string[]> {
  const left: string[] = [];
  for (const [i, c] of cases.entries()) {
    const id = (results[i] as { id?: string } | null)?.id;
    if (!id) continue;
    try {
      if (c.tool === "create_page") {
        const page = await figma.getNodeByIdAsync(id);
        if (page && page.type === "PAGE" && !page.removed) {
          if (figma.currentPage.id === page.id) await figma.setCurrentPageAsync(figma.root.children.find((p) => p.id !== page.id) as PageNode);
          page.remove();
        }
      } else if (c.tool === "create_variable_collection") {
        const figmaId = id.startsWith("VariableCollectionId:") ? id : `VariableCollectionId:${id}`;
        (await figma.variables.getVariableCollectionByIdAsync(figmaId))?.remove();
      } else if (c.tool === "create_style") {
        (await figma.getStyleByIdAsync(id))?.remove();
      }
    } catch (error) {
      left.push(`${c.tool} ${id}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return left;
}

export async function selftest(): Promise<{ rows: Row[]; left: string[] }> {
  const cases = recording.cases as Case[];
  const results: unknown[] = [];
  const rows: Row[] = [];
  let left: string[] = [];
  try {
    for (const [call, expected] of cases.entries()) {
      const row: Row = { call, tool: expected.tool, live: expected.live, outcome: "not run", notes: "", ms: 0 };
      rows.push(row);
      results.push(null);
      let args: Record<string, unknown>;
      try {
        args = substitute(expected.args, results) as Record<string, unknown>;
      } catch (error) {
        row.notes = error instanceof Error ? error.message : String(error);
        continue;
      }
      const done = REGISTRY[expected.tool];
      if (!done) {
        row.notes = "the panel has no function for it";
        continue;
      }
      const started = Date.now();
      try {
        const answer = await done(args);
        results[call] = answer;
        Object.assign(row, compare(expected, answer));
      } catch (error) {
        row.outcome = "refused";
        row.notes = error instanceof Error ? error.message : String(error);
      }
      row.ms = Date.now() - started;
    }
  } finally {
    left = await cleanUp(cases, results);
  }
  return { rows, left };
}

/** The table, as Markdown, for the ladder's README as it is. */
export function table(rows: Row[], left: string[]): string {
  const cell = (text: string) => text.replace(/\|/g, "\\|").replace(/\n/g, " ");
  const counts = ["same", "differs", "refused", "not run"].map((o) => `${rows.filter((r) => r.outcome === o).length} ${o}`).join(", ");
  const lines = [
    `Self-test on ${figma.root.name}, ${new Date().toISOString().slice(0, 10)}: ${counts}.`,
    "",
    "| # | Tool | Marked | In Figma | ms | Notes |",
    "|---|---|---|---|---|---|",
    ...rows.map((r) => `| ${r.call} | \`${r.tool}\` | ${r.live} | ${r.outcome} | ${r.ms} | ${cell(r.notes)} |`),
  ];
  for (const r of rows.filter((row) => row.all)) {
    lines.push("", `Call ${r.call}, \`${r.tool}\`, every difference:`, "", ...(r.all ?? []).map((d) => `- ${d}`));
  }
  if (left.length) lines.push("", `Not removed after: ${left.map(cell).join("; ")}`);
  return lines.join("\n");
}
