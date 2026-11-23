// Pages, sections, frames, texts, shapes and vectors made on the open file,
// each answered in the tool's result keys. A call that fails removes what it
// had made, so nothing is left half done.
import { Args, nodeFor, num } from "./nodes";
import { paintsIn } from "./paint";
import { Variables } from "./set";

type Parent = BaseNode & ChildrenMixin;

/** A page or a node that holds children, or words saying what it is instead. */
async function parentFor(id: unknown): Promise<Parent> {
  const node = await nodeFor(String(id));
  if (!("appendChild" in node)) throw new Error(`parent: ${node.id} is a ${node.type}, which holds no children`);
  return node as Parent;
}

/** What `created` answers on a file, without `unsaved_edits`: Figma saves it. */
function created(node: BaseNode): Record<string, unknown> {
  return { id: node.id, type: node.type, name: node.name };
}

/** Put `node` in `parent` at x, y, then run `rest`; if anything fails, the node goes. */
async function placed<T extends SceneNode>(node: T, parent: Parent, args: Args, rest: () => Promise<void> | void = () => {}): Promise<T> {
  try {
    parent.appendChild(node);
    node.x = typeof args.x === "number" ? args.x : 0;
    node.y = typeof args.y === "number" ? args.y : 0;
    await rest();
  } catch (error) {
    node.remove();
    throw error;
  }
  return node;
}

export async function createPage(args: Args): Promise<unknown> {
  const page = figma.createPage();
  page.name = String(args.name);
  return created(page);
}

export async function createSection(args: Args): Promise<unknown> {
  const page = await nodeFor(String(args.parent));
  if (page.type !== "PAGE") throw new Error(`parent: ${page.id} is a ${page.type}: a section is made on a page`);
  const section = figma.createSection();
  await placed(section, page, args, () => {
    section.name = typeof args.name === "string" ? args.name : "Section";
    section.resizeWithoutConstraints(Number(args.width), Number(args.height));
  });
  return created(section);
}

export async function createFrame(args: Args): Promise<unknown> {
  const parent = await parentFor(args.parent);
  const frame = figma.createFrame();
  await placed(frame, parent, args, () => {
    frame.name = String(args.name);
    // As on a file: an empty frame, with no fill.
    frame.fills = [];
    frame.resize(Number(args.width), Number(args.height));
  });
  return created(frame);
}

/** Shapes' own keys, refused on a kind that has none, as on a file. */
const OWN: Record<string, readonly string[]> = {
  arcData: ["ELLIPSE"],
  pointCount: ["POLYGON", "STAR"],
  innerRadius: ["STAR"],
  cornerRadius: ["RECTANGLE", "POLYGON", "STAR"],
};

const SHAPES: Record<string, { make: () => SceneNode & GeometryMixin & LayoutMixin; name: string }> = {
  RECTANGLE: { make: () => figma.createRectangle(), name: "Rectangle" },
  ELLIPSE: { make: () => figma.createEllipse(), name: "Ellipse" },
  LINE: { make: () => figma.createLine(), name: "Line" },
  POLYGON: { make: () => figma.createPolygon(), name: "Polygon" },
  STAR: { make: () => figma.createStar(), name: "Star" },
};

export async function createShape(args: Args): Promise<unknown> {
  const kind = String(args.type);
  const shape = SHAPES[kind];
  if (!shape) throw new Error(`parameter "type": \`${kind}\` is not one of ${Object.keys(SHAPES).join(", ")}`);
  for (const [key, kinds] of Object.entries(OWN)) {
    if (args[key] !== undefined && !kinds.includes(kind)) throw new Error(`${key}: a ${kind} has none; it is for ${kinds.join(", ")}`);
  }
  if (kind === "LINE") {
    if (args.fills !== undefined) throw new Error("fills: a LINE has no fill; give strokes");
    if (args.strokes === undefined) throw new Error("strokes: a LINE without strokes is invisible; give strokes");
    if (args.height !== undefined) throw new Error("height: a LINE has a length, its width, and no height");
  }
  const parent = await parentFor(args.parent);
  const variables = new Variables();
  // Checked before anything is made.
  const fills = args.fills === undefined ? [] : await paintsIn("fills", args.fills, (n) => variables.named(n));
  const strokes = args.strokes === undefined ? undefined : await paintsIn("strokes", args.strokes, (n) => variables.named(n));
  const node = shape.make();
  await placed(node, parent, args, () => {
    node.name = typeof args.name === "string" ? args.name : shape.name;
    node.resize(Number(args.width), kind === "LINE" ? 0 : Number(args.height));
    // As on a file: nothing is painted unless named.
    node.fills = fills;
    node.strokes = strokes ?? [];
    if (typeof args.strokeWeight === "number") node.strokeWeight = args.strokeWeight;
    const own = node as unknown as Record<string, unknown>;
    if (typeof args.cornerRadius === "number") own.cornerRadius = args.cornerRadius;
    if (args.arcData !== undefined) {
      const arc = args.arcData as Partial<ArcData>;
      own.arcData = { startingAngle: arc.startingAngle ?? 0, endingAngle: arc.endingAngle ?? 2 * Math.PI, innerRadius: arc.innerRadius ?? 0 };
    }
    if (typeof args.pointCount === "number") own.pointCount = args.pointCount;
    if (typeof args.innerRadius === "number") own.innerRadius = args.innerRadius;
  });
  return created(node);
}

/** The box SVG path data is drawn in: the viewBox given, or the size asked. */
function svgOf(args: Args): string {
  const paths = Array.isArray(args.path) ? args.path.map(String) : [String(args.path)];
  const rule = args.fillRule === "EVENODD" ? "evenodd" : "nonzero";
  const escape = (d: string) => d.replace(/[<>&"]/g, "");
  const box = Array.isArray(args.viewBox) ? (args.viewBox as number[]).join(" ") : undefined;
  const size = [typeof args.width === "number" ? `width="${args.width}"` : "", typeof args.height === "number" ? `height="${args.height}"` : ""].join(" ");
  const body = paths.map((d) => `<path d="${escape(d)}" fill="black" fill-rule="${rule}"/>`).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" ${box ? `viewBox="${box}"` : ""} ${size} preserveAspectRatio="none">${body}</svg>`;
}

/**
 * A vector from SVG path data: Figma reads the SVG itself, every command
 * included, and the paths are joined into one vector.
 */
export async function createVector(args: Args): Promise<unknown> {
  const parent = await parentFor(args.parent);
  const variables = new Variables();
  const fills = args.fills === undefined ? [] : await paintsIn("fills", args.fills, (n) => variables.named(n));
  // Figma's pen draws a black 1 px stroke; so does this, unless told.
  const strokes =
    args.strokes !== undefined
      ? await paintsIn("strokes", args.strokes, (n) => variables.named(n))
      : args.fills !== undefined
        ? []
        : [{ type: "SOLID", color: { r: 0, g: 0, b: 0 } } as SolidPaint];
  let frame: FrameNode;
  try {
    frame = figma.createNodeFromSvg(svgOf(args));
  } catch (error) {
    throw new Error(`path: Figma could not read it as SVG path data: ${error instanceof Error ? error.message : String(error)}`);
  }
  let vector: VectorNode;
  try {
    const drawn = frame.findAll((n) => n.type === "VECTOR") as VectorNode[];
    if (!drawn.length) throw new Error("path: Figma drew nothing from it");
    vector = drawn.length === 1 ? drawn[0] : (figma.flatten(drawn, frame) as VectorNode);
    // Where the drawing falls in the asked box, kept as it moves out of it.
    const at = { x: (typeof args.x === "number" ? args.x : 0) + vector.x, y: (typeof args.y === "number" ? args.y : 0) + vector.y };
    parent.appendChild(vector);
    vector.x = at.x;
    vector.y = at.y;
  } finally {
    frame.remove();
  }
  try {
    vector.name = typeof args.name === "string" ? args.name : "Vector";
    vector.fills = fills;
    vector.strokes = strokes;
    // As an SVG path is stroked, and as a file writes it: centred on the path.
    // Figma's SVG import leaves INSIDE, which draws nothing on an open path.
    vector.strokeAlign = "CENTER";
    vector.strokeWeight = typeof args.strokeWeight === "number" ? args.strokeWeight : 1;
    vector.strokeCap = (typeof args.strokeCap === "string" ? args.strokeCap : "NONE") as StrokeCap;
    vector.strokeJoin = (typeof args.strokeJoin === "string" ? args.strokeJoin : "MITER") as StrokeJoin;
  } catch (error) {
    vector.remove();
    throw error;
  }
  return { ...created(vector), size: { width: num(vector.width), height: num(vector.height) }, subpaths: vector.vectorPaths.length };
}

/** The most used font among the texts of the current page, and its most used size. */
function usualFont(): { font: FontName; size: number } | undefined {
  const fonts = new Map<string, { font: FontName; count: number; sizes: Map<number, number> }>();
  const texts = figma.currentPage.findAllWithCriteria({ types: ["TEXT"] }).slice(0, 2000);
  for (const text of texts) {
    if (text.fontName === figma.mixed || text.fontSize === figma.mixed) continue;
    const key = `${text.fontName.family}\u0000${text.fontName.style}`;
    const entry = fonts.get(key) ?? { font: text.fontName, count: 0, sizes: new Map() };
    entry.count += 1;
    entry.sizes.set(text.fontSize, (entry.sizes.get(text.fontSize) ?? 0) + 1);
    fonts.set(key, entry);
  }
  const most = [...fonts.values()].sort((a, b) => b.count - a.count)[0];
  if (!most) return undefined;
  const size = [...most.sizes.entries()].sort((a, b) => b[1] - a[1])[0][0];
  return { font: most.font, size };
}

type Run = {
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

const RUN_KEYS = new Set(["characters", "fontFamily", "fontStyle", "fontSize", "fills", "textDecoration", "textCase", "letterSpacing", "lineHeight", "hyperlink"]);

const LAYER_KEYS = [
  "textAlignHorizontal", "textAlignVertical", "textDecoration", "textCase", "paragraphSpacing", "paragraphIndent",
] as const;

function dimension(value: unknown): unknown {
  return typeof value === "number" ? { value, unit: "PIXELS" } : value;
}

export async function createText(args: Args): Promise<unknown> {
  const parent = await parentFor(args.parent);
  const runs = Array.isArray(args.runs) ? (args.runs as Run[]) : undefined;
  runs?.forEach((run, i) => {
    const extra = Object.keys(run).find((key) => !RUN_KEYS.has(key));
    if (extra) throw new Error(`runs[${i}]: \`${extra}\` is not accepted; a run takes ${[...RUN_KEYS].join(", ")}`);
    if (typeof run.characters !== "string" || !run.characters) throw new Error(`runs[${i}].characters: a run needs its text`);
  });
  const characters = runs ? runs.map((r) => r.characters).join("") : String(args.characters);
  const usual = usualFont();
  const font: FontName = {
    family: typeof args.fontFamily === "string" ? args.fontFamily : (usual?.font.family ?? "Inter"),
    style: typeof args.fontStyle === "string" ? args.fontStyle : typeof args.fontFamily === "string" ? "Regular" : (usual?.font.style ?? "Regular"),
  };
  try {
    await figma.loadFontAsync(font);
  } catch (error) {
    throw new Error(`fontFamily: Figma has no ${font.family} ${font.style} here: ${error instanceof Error ? error.message : String(error)}`);
  }
  const variables = new Variables();
  const black: SolidPaint = { type: "SOLID", color: { r: 0, g: 0, b: 0 } };
  const fills = args.fills === undefined ? [black] : await paintsIn("fills", args.fills, (n) => variables.named(n));
  const runFonts: FontName[] = [];
  for (const run of runs ?? []) {
    if (run.fontFamily === undefined && run.fontStyle === undefined) continue;
    const f = { family: run.fontFamily ?? font.family, style: run.fontStyle ?? (run.fontFamily ? "Regular" : font.style) };
    await figma.loadFontAsync(f);
    runFonts.push(f);
  }
  const text = figma.createText();
  await placed(text, parent, args, async () => {
    text.fontName = font;
    const size = typeof args.fontSize === "number" ? args.fontSize : usual && usual.font.family === font.family ? usual.size : undefined;
    if (size !== undefined) text.fontSize = size;
    text.characters = characters;
    text.name = typeof args.name === "string" ? args.name : layerName(characters);
    text.fills = fills;
    if (args.lineHeight !== undefined) text.lineHeight = dimension(args.lineHeight) as LineHeight;
    if (args.letterSpacing !== undefined) text.letterSpacing = dimension(args.letterSpacing) as LetterSpacing;
    const own = text as unknown as Record<string, unknown>;
    for (const key of LAYER_KEYS) if (args[key] !== undefined) own[key] = args[key];
    // As on a file: a height fixes the box; a width alone lets it grow down.
    const [width, height] = [args.width, args.height];
    if (typeof height === "number") {
      text.textAutoResize = "NONE";
      text.resize(typeof width === "number" ? width : text.width, height);
    } else if (typeof width === "number") {
      text.textAutoResize = "HEIGHT";
      text.resize(width, text.height);
    }
    if (typeof args.textAutoResize === "string") text.textAutoResize = args.textAutoResize as TextNode["textAutoResize"];
    if (runs) await styleRuns(text, runs, font, variables);
    if (Array.isArray(args.paragraphs)) paragraphs(text, args.paragraphs as { listType?: string; indentation?: number }[]);
  });
  const shown = text.fontName === figma.mixed ? font : text.fontName;
  return {
    ...created(text),
    font: { family: shown.family, style: shown.style, size: text.fontSize === figma.mixed ? null : num(text.fontSize) },
    size: { width: num(text.width), height: num(text.height) },
  };
}

/** Each run styled apart from the layer, by its range. */
async function styleRuns(text: TextNode, runs: Run[], layer: FontName, variables: Variables): Promise<void> {
  let start = 0;
  for (const [i, run] of runs.entries()) {
    const end = start + run.characters.length;
    const at = (key: string, step: () => Promise<void> | void) =>
      Promise.resolve()
        .then(step)
        .catch((error: unknown) => {
          throw new Error(`runs[${i}].${key}: ${error instanceof Error ? error.message : String(error)}`);
        });
    if (run.fontFamily !== undefined || run.fontStyle !== undefined) {
      const font = { family: run.fontFamily ?? layer.family, style: run.fontStyle ?? (run.fontFamily ? "Regular" : layer.style) };
      await at("fontFamily", () => text.setRangeFontName(start, end, font));
    }
    if (run.fontSize !== undefined) await at("fontSize", () => text.setRangeFontSize(start, end, Number(run.fontSize)));
    if (run.fills !== undefined) {
      const fills = await paintsIn(`runs[${i}].fills`, run.fills, (n) => variables.named(n));
      await at("fills", () => text.setRangeFills(start, end, fills));
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

/** One paragraph a line: a list's kind and its indentation. */
function paragraphs(text: TextNode, given: { listType?: string; indentation?: number }[]): void {
  const lines = text.characters.split("\n");
  if (given.length !== lines.length) {
    throw new Error(`paragraphs: ${given.length} given for ${lines.length} lines of text; give one a line`);
  }
  let start = 0;
  for (const [i, line] of lines.entries()) {
    const end = start + line.length;
    const paragraph = given[i];
    if (end > start) {
      if (paragraph.listType !== undefined) text.setRangeListOptions(start, end, { type: paragraph.listType as "NONE" | "ORDERED" | "UNORDERED" });
      if (paragraph.indentation !== undefined) text.setRangeIndentation(start, end, paragraph.indentation);
    }
    start = end + 1;
  }
}

/** A text's layer name by default: its first line, up to 40 characters. */
function layerName(characters: string): string {
  const first = characters.split("\n")[0];
  const chars = [...first];
  return chars.length > 40 ? `${chars.slice(0, 40).join("")}…` : first;
}
