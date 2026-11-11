// The page's states and their moves (internal: specs/109-open-in-browser/data-model.md).
import type { Summary } from "./protocol";

/** A file larger than this is refused before it is read (row 102). */
export const MAX_BYTES = 512 * 1024 * 1024;

export type State =
  | { kind: "empty" }
  | { kind: "loading"; name: string }
  | { kind: "reading"; name: string }
  | { kind: "read"; summary: Summary }
  | { kind: "refused"; name: string; message: string };

export type Action =
  | { type: "chose"; name: string; bytes: number; readerReady: boolean }
  | { type: "ready" }
  | { type: "opened"; summary: Summary }
  | { type: "failed"; message: string }
  | { type: "stopped" };

export function reduce(state: State, action: Action): State {
  switch (action.type) {
    case "chose":
      if (action.bytes > MAX_BYTES) {
        return {
          kind: "refused",
          name: action.name,
          message: `${action.name} is ${megabytes(action.bytes)}; a browser reads files up to ${megabytes(MAX_BYTES)}. Install grafta to read it.`,
        };
      }
      if (action.bytes === 0)
        return { kind: "refused", name: action.name, message: `${action.name} is empty.` };
      return action.readerReady
        ? { kind: "reading", name: action.name }
        : { kind: "loading", name: action.name };
    case "ready":
      return state.kind === "loading" ? { kind: "reading", name: state.name } : state;
    case "opened":
      return state.kind === "loading" || state.kind === "reading"
        ? { kind: "read", summary: action.summary }
        : state;
    case "failed":
      return state.kind === "loading" || state.kind === "reading"
        ? { kind: "refused", name: state.name, message: action.message }
        : state;
    case "stopped":
      return { kind: "empty" };
  }
}

/** The pages a person sees in Figma, and how many are only Figma's own. */
export function visiblePages(summary: Summary): { pages: Summary["pages"]; internal: number } {
  const pages = summary.pages.filter((page) => !page.internalOnly);
  return { pages, internal: summary.pages.length - pages.length };
}

export function megabytes(bytes: number): string {
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

const PRODUCTS: Record<Summary["product"], string> = {
  design: "Figma Design",
  slides: "Figma Slides",
  whiteboard: "FigJam",
  site: "Figma Sites",
};

export const productName = (product: Summary["product"]): string => PRODUCTS[product];

/** Characters a file name may not hold on some system. */
// eslint-disable-next-line no-control-regex -- control characters are what it removes
const NOT_IN_FILE_NAMES = /[/\\:*?"<>|\u0000-\u001f\u2028\u2029]/g;

/** A screen's name as a file name, with `@2x` when scaled. */
export function pngName(screen: string, scale: number): string {
  const name = screen.replace(NOT_IN_FILE_NAMES, "_").trim() || "screen";
  return scale === 1 ? `${name}.png` : `${name}@${scale}x.png`;
}

/** Names made unique within one archive: a second "Frame" is "Frame 2". */
export function unique(names: string[]): string[] {
  const seen = new Map<string, number>();
  return names.map((name) => {
    const count = (seen.get(name) ?? 0) + 1;
    seen.set(name, count);
    if (count === 1) return name;
    const dot = name.lastIndexOf(".");
    return dot > 0 ? `${name.slice(0, dot)} ${count}${name.slice(dot)}` : `${name} ${count}`;
  });
}

/** An archive's name: the file's, without its extension, and what is in it. */
export function archiveName(file: string, what: string): string {
  return `${file.replace(/\.[^.]+$/, "").replace(NOT_IN_FILE_NAMES, "_")} – ${what.replace(NOT_IN_FILE_NAMES, "_")}.zip`;
}
