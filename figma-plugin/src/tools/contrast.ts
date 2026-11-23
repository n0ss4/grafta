// check_contrast on the open file: what is behind each text, as Figma itself
// draws it, and each text as grafta measures it. The node is copied, the
// copy's texts made transparent, the copy drawn by Figma and removed; grafta reads each
// text's box in that picture with the same code as on a file.
import { Args, nodeFor } from "./nodes";

type Matrix = [number, number, number, number, number, number];

/** Pixels per unit the background is drawn at, as on a file, unless the node is too large for it. */
const SCALE = 2;
/** The largest side of the picture, in pixels. */
const LARGEST = 4096;

const NOT_SOLID = "its fill is not a solid colour, so it has no one colour to measure";

export async function checkContrast(args: Args): Promise<unknown> {
  const node = await nodeFor(String(args.id));
  if (!("exportAsync" in node) || node.type === "PAGE") throw new Error(`${node.id} is a ${node.type}, which Figma does not draw`);
  const drawn = node as SceneNode;
  const box = drawn.absoluteBoundingBox;
  if (!box || box.width <= 0 || box.height <= 0) throw new Error(`${node.id} has no area to draw`);
  const scale = Math.min(SCALE, LARGEST / Math.max(box.width, box.height));
  const origin: Matrix = [1, 0, 0, 1, -box.x, -box.y];

  const texts: TextNode[] = [];
  if (drawn.type === "TEXT") texts.push(drawn);
  else if ("findAllWithCriteria" in drawn) texts.push(...(drawn as ChildrenMixin).findAllWithCriteria({ types: ["TEXT"] }));
  const shown = texts.filter((text) => chain(text, drawn).every((n) => (n as SceneNode).visible !== false));
  const samples = [];
  for (const text of shown) samples.push(await sample(text, drawn, origin));

  const picture = await behind(drawn, scale);
  return { picture: figma.base64Encode(picture), scale, texts: samples };
}

/** The node and its ancestors up to `top`, `top` included. */
function chain(node: BaseNode, top: BaseNode): BaseNode[] {
  const out: BaseNode[] = [];
  for (let at: BaseNode | null = node; at; at = at.parent) {
    out.push(at);
    if (at === top) break;
  }
  return out;
}

/** `a` then `b`: the matrix of `b` inside `a`, as [a, b, c, d, e, f] in SVG's order. */
function compose(a: Matrix, b: Matrix): Matrix {
  return [
    a[0] * b[0] + a[2] * b[1],
    a[1] * b[0] + a[3] * b[1],
    a[0] * b[2] + a[2] * b[3],
    a[1] * b[2] + a[3] * b[3],
    a[0] * b[4] + a[2] * b[5] + a[4],
    a[1] * b[4] + a[3] * b[5] + a[5],
  ];
}

/** A text as grafta measures it: its box in the picture's space, its colours as Figma resolves them, its opacity. */
async function sample(text: TextNode, top: SceneNode, origin: Matrix): Promise<Record<string, unknown>> {
  const [[a, c, e], [b, d, f]] = text.absoluteTransform;
  const ancestors = chain(text, top);
  const opacity = ancestors.reduce((product, n) => product * ("opacity" in n ? (n as BlendMixin).opacity : 1), 1);
  const instance = ancestors.slice(1).find((n) => n.type === "INSTANCE");
  const runs: number[][] = [];
  let unmeasurable: string | undefined;
  if (text.characters.length) {
    for (const segment of text.getStyledTextSegments(["fills", "fontSize", "fontWeight"])) {
      // Only what is drawn is measured: a run over nothing but spaces shows no colour.
      if (!segment.characters.trim()) continue;
      for (const paint of segment.fills.filter((p) => p.visible !== false)) {
        if (paint.type !== "SOLID") {
          unmeasurable = NOT_SOLID;
          continue;
        }
        const colour = await resolved(paint, text);
        runs.push([colour.r, colour.g, colour.b, colour.a * (paint.opacity ?? 1), segment.fontSize, segment.fontWeight]);
      }
    }
  }
  if (!runs.length && !unmeasurable) unmeasurable = "it has no visible fill";
  const out: Record<string, unknown> = {
    id: text.id,
    name: text.name,
    characters: text.characters,
    at: compose(origin, [a, b, c, d, e, f]),
    width: text.width,
    height: text.height,
    opacity,
    runs,
  };
  if (instance) out.instance = instance.id;
  if (unmeasurable) out.unmeasurable = unmeasurable;
  return out;
}

/** A solid paint's colour where `text` is drawn: a bound variable's value in the modes in effect there. */
async function resolved(paint: SolidPaint, text: TextNode): Promise<RGBA> {
  const alias = paint.boundVariables?.color;
  if (alias) {
    const variable = await figma.variables.getVariableByIdAsync(alias.id);
    const value = variable?.resolveForConsumer(text).value;
    if (typeof value === "object" && value !== null && "r" in value) return { r: value.r, g: value.g, b: value.b, a: "a" in value ? value.a : 1 };
  }
  return { ...paint.color, a: 1 };
}

/** The node drawn by Figma with its texts hidden, from a copy in the node's page, removed after. */
async function behind(node: SceneNode, scale: number): Promise<Uint8Array> {
  let page: BaseNode | null = node.parent;
  while (page && page.type !== "PAGE") page = page.parent;
  const copy = node.clone();
  try {
    if (page) (page as PageNode).appendChild(copy);
    // The modes the node is shown in, inherited ones included, kept on the copy now outside its parents.
    if ("setExplicitVariableModeForCollection" in copy) {
      for (const [collection, mode] of Object.entries(node.resolvedVariableModes)) {
        const found = await figma.variables.getVariableCollectionByIdAsync(collection);
        if (found) (copy as SceneNode & ExplicitVariableModesMixin).setExplicitVariableModeForCollection(found, mode);
      }
    }
    // Transparent, not hidden: a hidden layer leaves an auto-layout's flow, and what follows it moves up.
    if (copy.type === "TEXT") copy.opacity = 0;
    else if ("findAllWithCriteria" in copy) for (const text of (copy as ChildrenMixin).findAllWithCriteria({ types: ["TEXT"] })) text.opacity = 0;
    const box = node.absoluteBoundingBox;
    const copied = copy.absoluteBoundingBox;
    if (!box || !copied || Math.abs(box.width - copied.width) > 0.01 || Math.abs(box.height - copied.height) > 0.01) {
      throw new Error(`a copy of ${node.id} outside its parent is not its size (${copied?.width}×${copied?.height} against ${box?.width}×${box?.height}), so what is behind its texts cannot be drawn`);
    }
    return await copy.exportAsync({ format: "PNG", constraint: { type: "SCALE", value: scale }, useAbsoluteBounds: true });
  } finally {
    copy.remove();
  }
}
