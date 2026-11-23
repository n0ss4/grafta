// What a call changed, shown to the person: the layer it changed, followed on
// the canvas when they want, and each property it changed, before and after.
// Read with read_node's own view (tools/view.ts), defaults left out, so a
// property set back to Figma's default reads "default". Nothing here writes
// to the file: following a change is a selection and the viewport.
import type { Change } from "./protocol";
import { Names } from "./tools/nodes";
import { view } from "./tools/view";

type Args = Record<string, unknown>;
type View = Record<string, unknown>;

/** The most properties shown for one call; the rest are counted. */
const SHOWN = 24;

/** The layer a call works on, read before it runs, or nothing for a new one. */
export async function snapshot(args: Args): Promise<View | undefined> {
  if (typeof args.id !== "string") return undefined;
  try {
    const node = await figma.getNodeByIdAsync(args.id);
    return node ? await read(node) : undefined;
  } catch {
    return undefined;
  }
}

/** The layer a call changed or made: its result's id, else the one it was given. */
export function target(args: Args, result: unknown): string | undefined {
  const made = (result as { id?: unknown } | undefined)?.id;
  if (typeof made === "string") return made;
  return typeof args.id === "string" ? args.id : undefined;
}

/** Each property that differs between the layer before and after, in its view's order. */
export async function changes(id: string | undefined, before: View | undefined): Promise<{ changes: Change[]; more: number }> {
  if (!id || !before) return { changes: [], more: 0 };
  const node = await figma.getNodeByIdAsync(id).catch(() => null);
  if (!node) return { changes: [], more: 0 };
  const after = flat(await read(node));
  const was = flat(before);
  const keys = [...new Set([...Object.keys(was), ...Object.keys(after)])].filter((k) => !["id", "path", "childCount"].includes(k) && !k.startsWith("children"));
  const all = keys.filter((k) => was[k] !== after[k]).map((k) => ({ property: label(k), before: was[k] ?? DEFAULT, after: after[k] ?? DEFAULT }));
  return { changes: all.slice(0, SHOWN), more: Math.max(0, all.length - SHOWN) };
}

/** Select the layer and bring it into view, on its own page. */
export async function focus(id: string): Promise<boolean> {
  const node = await figma.getNodeByIdAsync(id).catch(() => null);
  if (!node || node.type === "DOCUMENT" || node.type === "PAGE" || node.removed) return false;
  let page: BaseNode | null = node.parent;
  while (page && page.type !== "PAGE") page = page.parent;
  if (page && page.type === "PAGE" && page !== figma.currentPage) await figma.setCurrentPageAsync(page);
  figma.currentPage.selection = [node as SceneNode];
  figma.viewport.scrollAndZoomIntoView([node as SceneNode]);
  return true;
}

const DEFAULT = "default";

async function read(node: BaseNode): Promise<View> {
  return view(node, { depth: 0, budget: 1 << 20, geometry: false, childrenFrom: 0 }, new Names());
}

/** A view as one value per property path, each said as the person would read it. */
function flat(value: unknown, path = "", out: Record<string, string> = {}): Record<string, string> {
  if (Array.isArray(value)) {
    if (value.length === 0 && path) out[path] = "none";
    value.forEach((item, i) => flat(item, `${path}.${i}`, out));
  } else if (value && typeof value === "object") {
    const object = value as Record<string, unknown>;
    if ("unit" in object && Object.keys(object).every((k) => k === "unit" || k === "value")) {
      out[path] = measure(object);
    } else {
      for (const [key, item] of Object.entries(object)) flat(item, path ? `${path}.${key}` : key, out);
    }
  } else if (path) {
    out[path] = typeof value === "number" ? String(Math.round(value * 100) / 100) : String(value);
  }
  return out;
}

/** `{ unit, value }` as Figma's panel writes it: 150%, 48px, Auto. */
function measure(m: Record<string, unknown>): string {
  if (m.unit === "AUTO") return "Auto";
  const value = typeof m.value === "number" ? Math.round(m.value * 100) / 100 : m.value;
  return m.unit === "PERCENT" ? `${value}%` : m.unit === "PIXELS" ? `${value}px` : `${value}`;
}

/** Figma's panel's own words for a property path. */
const WORDS: Record<string, string> = {
  x: "X",
  y: "Y",
  width: "Width",
  height: "Height",
  rotation: "Rotation",
  opacity: "Opacity",
  visible: "Visible",
  name: "Name",
  characters: "Text",
  cornerRadius: "Corner radius",
  strokeWeight: "Stroke weight",
  layoutMode: "Auto layout",
  itemSpacing: "Gap",
  primaryAxisAlignItems: "Alignment",
  counterAxisAlignItems: "Alignment",
  layoutSizingHorizontal: "Width sizing",
  layoutSizingVertical: "Height sizing",
  textStyle: "Text style",
  fillStyle: "Fill style",
  fontFamily: "Font",
  fontStyle: "Font style",
  fontSize: "Font size",
  lineHeight: "Line height",
  letterSpacing: "Letter spacing",
  textAlignHorizontal: "Text align",
  textAutoResize: "Resizing",
};

/** `fills.0.color` as "Fill", `style.fontSize` as "Font size", `paddingTop` as "Padding top". */
function label(path: string): string {
  const parts = path.split(".");
  const [head] = parts;
  const tail = parts[parts.length - 1];
  const list: Record<string, string> = { fills: "Fill", strokes: "Stroke", effects: "Effect" };
  if (list[head]) {
    const index = parts.find((p) => /^\d+$/.test(p));
    const which = index && index !== "0" ? ` ${Number(index) + 1}` : "";
    const what = tail === "color" || /^\d+$/.test(tail) || tail === head ? "" : ` ${words(tail).toLowerCase()}`;
    return `${list[head]}${which}${what}`;
  }
  return WORDS[tail] ?? words(tail);
}

function words(key: string): string {
  const spaced = key.replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/** set_properties' shorthands, read as the view names them. */
const ALIASES: Record<string, string[]> = { padding: ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"] };

/** What set_properties is asked to change on a layer, before it runs: each property now and asked. */
export function planned(base: View, properties: unknown): Change[] {
  if (!properties || typeof properties !== "object") return [];
  const was = flat(base);
  const style = new Set(["fontFamily", "fontStyle", "fontSize", "lineHeight", "letterSpacing", "textAlignHorizontal", "textAlignVertical", "textAutoResize"]);
  const out: Change[] = [];
  for (const [key, value] of Object.entries(properties as Record<string, unknown>)) {
    const asked = flat(value, key);
    for (const [path, after] of Object.entries(asked)) {
      if (path.endsWith(".type") && after === "SOLID") continue;
      const paths = ALIASES[path] ?? [style.has(path.split(".")[0]) ? `style.${path}` : path];
      const before = was[paths[0]] ?? DEFAULT;
      if (before === after) continue;
      out.push({ property: label(path), before, after });
    }
  }
  return out.slice(0, SHOWN);
}
