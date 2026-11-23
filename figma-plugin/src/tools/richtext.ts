// Rich text on the open file: runs styled apart and paragraphs as lists, as
// create_text and set_properties take them on a file, and what a text held
// before, kept so that a refused call can put it back.
import { inLayersMode, paintsIn } from "./paint";

export type Run = {
  characters: string;
  fontFamily?: string;
  fontStyle?: string;
  fontSize?: number;
  fills?: unknown;
  textDecoration?: TextDecoration;
  textCase?: TextCase;
  letterSpacing?: number | LetterSpacing;
  lineHeight?: number | LineHeight;
  hyperlink?: HyperlinkTarget;
};

type Named = (name: string) => Promise<Variable>;

const RUN_KEYS = new Set(["characters", "fontFamily", "fontStyle", "fontSize", "fills", "textDecoration", "textCase", "letterSpacing", "lineHeight", "hyperlink"]);

const words = (error: unknown): string => (error instanceof Error ? error.message : String(error));

export function dimension(value: unknown): unknown {
  return typeof value === "number" ? { value, unit: "PIXELS" } : value;
}

/** `runs` checked before anything changes, as a file checks them. */
export function runsIn(value: unknown): Run[] {
  if (!Array.isArray(value) || !value.length) throw new Error("runs: expected a non-empty array of runs");
  value.forEach((run: Record<string, unknown>, i) => {
    if (typeof run !== "object" || run === null) throw new Error(`runs[${i}]: expected an object`);
    const extra = Object.keys(run).find((key) => !RUN_KEYS.has(key));
    if (extra) throw new Error(`runs[${i}]: \`${extra}\` is not accepted; a run takes ${[...RUN_KEYS].join(", ")}`);
    if (typeof run.characters !== "string" || !run.characters) throw new Error(`runs[${i}].characters: a run needs its text`);
  });
  return value as Run[];
}

/** Each run styled apart from the layer, by its range; its font loaded first. */
export async function styleRuns(text: TextNode, runs: Run[], layer: FontName, named: Named): Promise<void> {
  let start = 0;
  for (const [i, run] of runs.entries()) {
    const end = start + run.characters.length;
    const at = (key: string, step: () => Promise<void> | void) =>
      Promise.resolve()
        .then(step)
        .catch((error: unknown) => {
          throw new Error(`runs[${i}].${key}: ${words(error)}`);
        });
    if (run.fontFamily !== undefined || run.fontStyle !== undefined) {
      const font = { family: run.fontFamily ?? layer.family, style: run.fontStyle ?? (run.fontFamily ? "Regular" : layer.style) };
      await at(run.fontFamily !== undefined ? "fontFamily" : "fontStyle", async () => {
        await figma.loadFontAsync(font);
        text.setRangeFontName(start, end, font);
      });
    }
    if (run.fontSize !== undefined) await at("fontSize", () => text.setRangeFontSize(start, end, Number(run.fontSize)));
    if (run.fills !== undefined) {
      const fills = await paintsIn(`runs[${i}].fills`, run.fills, named);
      await at("fills", () => text.setRangeFills(start, end, inLayersMode(text, fills)));
    }
    if (run.textCase !== undefined) await at("textCase", () => text.setRangeTextCase(start, end, run.textCase as TextCase));
    if (run.letterSpacing !== undefined) await at("letterSpacing", () => text.setRangeLetterSpacing(start, end, dimension(run.letterSpacing) as LetterSpacing));
    if (run.lineHeight !== undefined) await at("lineHeight", () => text.setRangeLineHeight(start, end, dimension(run.lineHeight) as LineHeight));
    if (run.hyperlink !== undefined) {
      await at("hyperlink", () => text.setRangeHyperlink(start, end, run.hyperlink as HyperlinkTarget));
      // As on a file: a link is underlined unless it says otherwise.
      if (run.textDecoration === undefined) await at("hyperlink", () => text.setRangeTextDecoration(start, end, "UNDERLINE"));
    }
    if (run.textDecoration !== undefined) await at("textDecoration", () => text.setRangeTextDecoration(start, end, run.textDecoration as TextDecoration));
    start = end;
  }
}

type Paragraph = { listType: "NONE" | "ORDERED" | "UNORDERED"; indentation: number };

/** `paragraphs` checked against the text's lines, as a file checks them: a list is indented 1 unless it says 2. */
export function paragraphsIn(characters: string, value: unknown): Paragraph[] {
  if (!Array.isArray(value)) throw new Error("paragraphs: expected an array");
  const count = characters.split("\n").length;
  if (value.length !== count) throw new Error(`paragraphs: ${value.length} given for ${count} lines of text; give one a line`);
  return value.map((item: Record<string, unknown>, i): Paragraph => {
    const at = `paragraphs[${i}]`;
    if (typeof item !== "object" || item === null) throw new Error(`${at}: expected an object`);
    const extra = Object.keys(item).find((key) => key !== "listType" && key !== "indentation");
    if (extra) throw new Error(`${at}.${extra}: a paragraph takes listType and indentation`);
    const listType = item.listType ?? "NONE";
    if (listType !== "NONE" && listType !== "ORDERED" && listType !== "UNORDERED") {
      throw new Error(`${at}.listType: expected NONE, UNORDERED or ORDERED, got ${JSON.stringify(listType)}`);
    }
    const given = item.indentation;
    if (listType === "NONE") {
      if (given !== undefined && given !== 0) throw new Error(`${at}.indentation: a paragraph that is not a list is not indented, got ${JSON.stringify(given)}`);
      return { listType, indentation: 0 };
    }
    if (given !== undefined && given !== 1 && given !== 2) throw new Error(`${at}.indentation: expected 1 or 2, got ${JSON.stringify(given)}`);
    return { listType, indentation: (given as number | undefined) ?? 1 };
  });
}

/** One paragraph a line: a list's kind and its indentation, each line set, so a list given NONE stops being one. */
export function setParagraphs(text: TextNode, given: Paragraph[]): void {
  let start = 0;
  for (const [i, line] of text.characters.split("\n").entries()) {
    const end = start + line.length;
    if (end > start) {
      text.setRangeListOptions(start, end, { type: given[i].listType });
      text.setRangeIndentation(start, end, given[i].indentation);
    }
    start = end + 1;
  }
}

const SEGMENT_FIELDS = [
  "fontName", "fontSize", "fills", "textDecoration", "textCase", "letterSpacing", "lineHeight", "hyperlink",
  "listOptions", "indentation", "fillStyleId", "textStyleId",
] as const;

type Segment = Pick<StyledTextSegment, "start" | "end" | (typeof SEGMENT_FIELDS)[number]>;

/** A range given a segment's styles, as getStyledTextSegments read them. */
async function restyle(text: TextNode, s: Segment, a: number, b: number): Promise<void> {
  // A text style gives the range its type; setting it apart would detach it.
  if (s.textStyleId) {
    await text.setRangeTextStyleIdAsync(a, b, s.textStyleId);
  } else {
    text.setRangeFontName(a, b, s.fontName);
    text.setRangeFontSize(a, b, s.fontSize);
    text.setRangeTextDecoration(a, b, s.textDecoration);
    text.setRangeTextCase(a, b, s.textCase);
    text.setRangeLetterSpacing(a, b, s.letterSpacing);
    text.setRangeLineHeight(a, b, s.lineHeight);
  }
  text.setRangeHyperlink(a, b, s.hyperlink);
  text.setRangeListOptions(a, b, s.listOptions);
  text.setRangeIndentation(a, b, s.indentation);
  if (s.fillStyleId) await text.setRangeFillStyleIdAsync(a, b, s.fillStyleId);
  else text.setRangeFills(a, b, s.fills);
}

/** The text's characters and their styles as they are, and how to put them back. */
export function kept(text: TextNode): () => Promise<void> {
  const characters = text.characters;
  const segments = characters.length ? text.getStyledTextSegments([...SEGMENT_FIELDS]) : [];
  return async () => {
    const fonts = segments.map((s) => s.fontName);
    if (text.characters.length) fonts.push(...text.getRangeAllFontNames(0, text.characters.length));
    await Promise.all(fonts.map((font) => figma.loadFontAsync(font)));
    text.characters = characters;
    for (const s of segments) await restyle(text, s, s.start, s.end);
  };
}

/**
 * New characters in the layer's style, as its first character holds it, a
 * link and a list left out: what a file gives characters no run styles apart.
 * The layer's font is answered, for the runs to fall back on.
 */
export async function replaced(text: TextNode, characters: string): Promise<FontName> {
  const [first] = text.characters.length
    ? text.getStyledTextSegments([...SEGMENT_FIELDS], 0, 1)
    : [];
  text.characters = characters;
  if (!first) return text.fontName as FontName;
  const plain = { ...first, hyperlink: null, listOptions: { type: "NONE" as const }, indentation: 0 };
  await restyle(text, plain, 0, characters.length);
  return first.fontName;
}
