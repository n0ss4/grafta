// Pages, sections, frames, texts, shapes and vectors made on the open file,
// each answered in the tool's result keys. A call that fails removes what it
// had made, so nothing is left half done.
import { Args, nodeFor, num } from "./nodes";
import { paintsIn, setPaints } from "./paint";
import { dimension, paragraphsIn, runsIn, setParagraphs, styleRuns } from "./richtext";
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
    setPaints(node as unknown as Record<string, unknown>, "fills", fills);
    setPaints(node as unknown as Record<string, unknown>, "strokes", strokes ?? []);
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

type Box = [number, number, number, number];

/** The paths as SVG, drawn in `box` and scaled to `size` when both are given; as given otherwise. */
function svgOf(args: Args, box?: Box, size?: [number, number]): string {
  const paths = Array.isArray(args.path) ? args.path.map(String) : [String(args.path)];
  const rule = args.fillRule === "EVENODD" ? "evenodd" : "nonzero";
  const escape = (d: string) => d.replace(/[<>&"]/g, "");
  const view = box ? `viewBox="${box.join(" ")}"` : "";
  const sized = size ? `width="${size[0]}" height="${size[1]}"` : "";
  const body = paths.map((d) => `<path d="${escape(d)}" fill="black" fill-rule="${rule}"/>`).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" ${view} ${sized} preserveAspectRatio="none">${body}</svg>`;
}

function fromSvg(svg: string): FrameNode {
  try {
    return figma.createNodeFromSvg(svg);
  } catch (error) {
    throw new Error(`path: Figma could not read it as SVG path data: ${error instanceof Error ? error.message : String(error)}`);
  }
}

/** The box the paths are drawn in, as a file takes it: the viewBox given, or the drawing's own bounds, Figma's reading of them. */
function boxOf(args: Args): Box {
  if (Array.isArray(args.viewBox)) return args.viewBox as Box;
  const frame = fromSvg(svgOf(args));
  try {
    const drawn = frame.findAll((n) => n.type === "VECTOR") as VectorNode[];
    if (!drawn.length) throw new Error("path: Figma drew nothing from it");
    const [x0, y0] = [Math.min(...drawn.map((v) => v.x)), Math.min(...drawn.map((v) => v.y))];
    const [x1, y1] = [Math.max(...drawn.map((v) => v.x + v.width)), Math.max(...drawn.map((v) => v.y + v.height))];
    return [x0, y0, x1 - x0, y1 - y0];
  } finally {
    frame.remove();
  }
}

/** The size the box is drawn at, as a file takes it: width and height, or one and the box's ratio, or the box's own. */
function sizeOf(args: Args, box: Box): [number, number] {
  const [w, h] = [typeof args.width === "number" ? args.width : undefined, typeof args.height === "number" ? args.height : undefined];
  if (w !== undefined && h !== undefined) return [w, h];
  if (w !== undefined) return [w, (w * box[3]) / box[2]];
  if (h !== undefined) return [(h * box[2]) / box[3], h];
  return [box[2], box[3]];
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
  // As on a file: the box is scaled to the size asked, so a 24 px icon asked at 48 is drawn at 48.
  const box = boxOf(args);
  const frame = fromSvg(svgOf(args, box, sizeOf(args, box)));
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
    setPaints(vector as unknown as Record<string, unknown>, "fills", fills);
    setPaints(vector as unknown as Record<string, unknown>, "strokes", strokes);
    // As an SVG path is stroked, and as a file writes it: centred on the path.
    // Figma's SVG import leaves INSIDE, which draws nothing on an open path.
    vector.strokeAlign = "CENTER";
    // Figma's default, as a file writes none: the SVG import leaves SCALE, which stretches the icon with its parent.
    vector.constraints = { horizontal: "MIN", vertical: "MIN" };
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

const LAYER_KEYS = [
  "textAlignHorizontal", "textAlignVertical", "textDecoration", "textCase", "paragraphSpacing", "paragraphIndent",
] as const;

export async function createText(args: Args): Promise<unknown> {
  const parent = await parentFor(args.parent);
  const runs = args.runs === undefined ? undefined : runsIn(args.runs);
  const characters = runs ? runs.map((r) => r.characters).join("") : String(args.characters);
  const paragraphs = args.paragraphs === undefined ? undefined : paragraphsIn(characters, args.paragraphs);
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
    setPaints(text as unknown as Record<string, unknown>, "fills", fills);
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
    if (runs) await styleRuns(text, runs, font, (n) => variables.named(n));
    if (paragraphs) setParagraphs(text, paragraphs);
  });
  const shown = text.fontName === figma.mixed ? font : text.fontName;
  return {
    ...created(text),
    font: { family: shown.family, style: shown.style, size: text.fontSize === figma.mixed ? null : num(text.fontSize) },
    size: { width: num(text.width), height: num(text.height) },
  };
}

/** A text's layer name by default: its first line, up to 40 characters. */
function layerName(characters: string): string {
  const first = characters.split("\n")[0];
  const chars = [...first];
  return chars.length > 40 ? `${chars.slice(0, 40).join("")}…` : first;
}
