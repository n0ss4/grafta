// The file's colour, text and effect styles, on the open file.
import { Args, Names, num } from "./nodes";
import { paintsIn, paintsOut, setPaints } from "./paint";
import { effects, Variables } from "./set";
import { effect } from "./view";

type Kind = "FILL" | "TEXT" | "EFFECT";

const kindOf = (style: BaseStyle): Kind | undefined =>
  style.type === "PAINT" ? "FILL" : style.type === "TEXT" ? "TEXT" : style.type === "EFFECT" ? "EFFECT" : undefined;

async function local(kind?: Kind): Promise<BaseStyle[]> {
  const all: BaseStyle[] = [];
  if (!kind || kind === "FILL") all.push(...(await figma.getLocalPaintStylesAsync()));
  if (!kind || kind === "TEXT") all.push(...(await figma.getLocalTextStylesAsync()));
  if (!kind || kind === "EFFECT") all.push(...(await figma.getLocalEffectStylesAsync()));
  return all;
}

async function styleJson(style: BaseStyle, names: Names): Promise<Record<string, unknown>> {
  const out: Record<string, unknown> = { id: style.id, name: style.name, type: kindOf(style) };
  if (style.type === "PAINT") out.fills = await paintsOut((style as PaintStyle).paints, names);
  if (style.type === "EFFECT") out.effects = (style as EffectStyle).effects.map(effect);
  if (style.type === "TEXT") {
    const text = style as TextStyle;
    out.fontFamily = text.fontName.family;
    out.fontStyle = text.fontName.style;
    out.fontSize = num(text.fontSize);
    if (text.lineHeight.unit !== "AUTO") out.lineHeight = { value: num(text.lineHeight.value), unit: text.lineHeight.unit };
    if (text.letterSpacing.value !== 0) out.letterSpacing = { value: num(text.letterSpacing.value), unit: text.letterSpacing.unit };
  }
  return out;
}

export async function listStyles(args: Args): Promise<unknown> {
  const names = new Names();
  const styles = [];
  for (const style of await local(args.type as Kind | undefined)) styles.push(await styleJson(style, names));
  return { total: styles.length, styles };
}

const PAINT_KEYS = ["fills"];
const EFFECT_KEYS = ["effects"];
const TEXT_KEYS = ["fontFamily", "fontStyle", "fontSize", "lineHeight", "letterSpacing"];

function keysFor(kind: Kind): string[] {
  return kind === "FILL" ? PAINT_KEYS : kind === "EFFECT" ? EFFECT_KEYS : TEXT_KEYS;
}

/** The values a style is given, written into it. */
async function write(style: BaseStyle, kind: Kind, args: Args): Promise<void> {
  const given = ["fills", "effects", ...TEXT_KEYS].filter((key) => args[key] !== undefined);
  const wrong = given.find((key) => !keysFor(kind).includes(key));
  if (wrong) throw new Error(`${wrong}: a ${kind} style takes ${keysFor(kind).join(", ")}`);
  if (kind === "FILL" && args.fills !== undefined) {
    const variables = new Variables();
    setPaints(style as unknown as Record<string, unknown>, "paints", await paintsIn("fills", args.fills, (name) => variables.named(name)));
  }
  if (kind === "EFFECT" && args.effects !== undefined) (style as EffectStyle).effects = effects(args.effects);
  if (kind === "TEXT") {
    const text = style as TextStyle;
    const font = { family: String(args.fontFamily ?? text.fontName.family), style: String(args.fontStyle ?? text.fontName.style) };
    await figma.loadFontAsync(font);
    text.fontName = font;
    if (typeof args.fontSize === "number") text.fontSize = args.fontSize;
    for (const key of ["lineHeight", "letterSpacing"] as const) {
      const value = args[key];
      if (value === undefined) continue;
      const measure = (typeof value === "number" ? { value, unit: "PIXELS" } : value) as LineHeight & LetterSpacing;
      text[key] = measure;
    }
  }
}

export async function createStyle(args: Args): Promise<unknown> {
  const kind = String(args.type) as Kind;
  const style = kind === "FILL" ? figma.createPaintStyle() : kind === "TEXT" ? figma.createTextStyle() : figma.createEffectStyle();
  try {
    style.name = String(args.name);
    await write(style, kind, args);
  } catch (error) {
    style.remove();
    throw error;
  }
  return styleJson(style, new Names());
}

/** A local style by id, or by name when only one has it. */
async function styleNamed(wanted: string): Promise<BaseStyle> {
  const byId = await figma.getStyleByIdAsync(wanted);
  if (byId && !byId.remote) return byId;
  const found = (await local()).filter((s) => s.name === wanted);
  if (found.length > 1) throw new Error(`${found.length} styles are named \`${wanted}\`; pass one id: ${found.map((s) => s.id).join(", ")}`);
  if (!found.length) throw new Error(`no local style \`${wanted}\`: list_styles lists them`);
  return found[0];
}

export async function setStyle(args: Args): Promise<unknown> {
  const style = await styleNamed(String(args.id));
  const kind = kindOf(style);
  if (!kind) throw new Error(`${style.name} is a ${style.type} style, which grafta does not change`);
  if (typeof args.name === "string") style.name = args.name;
  await write(style, kind, args);
  const out = await styleJson(style, new Names());
  out.nodes_using_it = (await style.getStyleConsumersAsync()).length;
  return out;
}
