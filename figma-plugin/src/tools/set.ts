// set_properties on the open file. Each property is set by the plugin API,
// the value before kept; if Figma refuses one, those already set are put back
// and the refusal is answered in Figma's words, so that, as on a file,
// nothing changes if any value is refused.
import { swapTarget } from "./components";
import { Args, bare, isId, nodeFor, read } from "./nodes";
import { paintsIn, parseHex } from "./paint";
import { kept, paragraphsIn, replaced, runsIn, setParagraphs, styleRuns } from "./richtext";

type Undo = () => Promise<void> | void;
type Node = BaseNode & Record<string, unknown>;

/** Set as they are given, by the plugin API's own names. */
const PLAIN = new Set([
  "x", "y", "name", "visible", "clipsContent", "layoutMode", "primaryAxisSizingMode", "counterAxisSizingMode",
  "primaryAxisAlignItems", "counterAxisAlignItems", "layoutWrap", "layoutGrow", "layoutAlign", "layoutPositioning",
  "strokeWeight", "strokeAlign", "strokeCap", "strokeJoin", "dashPattern",
]);

/** Numbers that may come from a variable, by the field a binding names. */
const BINDABLE: Record<string, readonly string[]> = {
  opacity: ["opacity"],
  itemSpacing: ["itemSpacing"],
  counterAxisSpacing: ["counterAxisSpacing"],
  padding: ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"],
  paddingTop: ["paddingTop"],
  paddingRight: ["paddingRight"],
  paddingBottom: ["paddingBottom"],
  paddingLeft: ["paddingLeft"],
  cornerRadius: ["topLeftRadius", "topRightRadius", "bottomRightRadius", "bottomLeftRadius"],
  topLeftRadius: ["topLeftRadius"],
  topRightRadius: ["topRightRadius"],
  bottomRightRadius: ["bottomRightRadius"],
  bottomLeftRadius: ["bottomLeftRadius"],
};

/** A text's own, set once its fonts are loaded. */
const TEXT = new Set([
  "characters", "fontFamily", "fontStyle", "fontSize", "textAlignHorizontal", "textAlignVertical", "lineHeight",
  "letterSpacing", "textDecoration", "textCase", "textAutoResize", "paragraphSpacing", "paragraphIndent",
]);

const STYLES: Record<string, { set: string; id: string; kind: "PAINT" | "EFFECT" | "TEXT" }> = {
  fillStyle: { set: "setFillStyleIdAsync", id: "fillStyleId", kind: "PAINT" },
  strokeStyle: { set: "setStrokeStyleIdAsync", id: "strokeStyleId", kind: "PAINT" },
  effectStyle: { set: "setEffectStyleIdAsync", id: "effectStyleId", kind: "EFFECT" },
  textStyle: { set: "setTextStyleIdAsync", id: "textStyleId", kind: "TEXT" },
};

/** A text's runs and paragraphs, set after its layer's own keys. */
const RICH = new Set(["runs", "paragraphs"]);

/** What a file takes and the open file does not yet: refused by name. */
const NOT_BUILT = new Set([
  "componentPropertyReferences", "reactions", "scrollBehavior",
  "overflowDirection", "overlayPositionType", "overlayBackground", "overlayBackgroundInteraction", "flowStartingPoint",
  "transition", "objectAnimations", "isSkippedSlide", "speakerNotes", "text", "cells", "connectorStart", "connectorEnd",
  "connectorStartStrokeCap", "connectorEndStrokeCap", "connectorLineType",
]);

export async function setProperties(args: Args): Promise<unknown> {
  const node = (await nodeFor(String(args.id))) as Node;
  const properties = (args.properties ?? {}) as Record<string, unknown>;
  const keys = Object.keys(properties);
  if (!keys.length) throw new Error("nothing to change: pass properties");
  const unknown = keys.find((key) => NOT_BUILT.has(key));
  if (unknown) throw new Error(`properties.${unknown}: not built for the open file yet, and nothing was changed`);
  if ("characters" in properties && "runs" in properties) throw new Error("characters and runs: give the text one way, not both");
  const undo: Undo[] = [];
  const set: string[] = [];
  const variables = new Variables();
  try {
    if (keys.some((key) => TEXT.has(key))) await textChanges(node, properties, undo, variables);
    if ("runs" in properties || "paragraphs" in properties) await richText(node, properties, undo, variables);
    if ("width" in properties || "height" in properties) await size(node, properties, undo, variables);
    for (const key of keys) {
      if (TEXT.has(key) || RICH.has(key) || key === "width" || key === "height") continue;
      try {
        await one(node, key, properties[key], undo, variables);
      } catch (error) {
        throw new Error(`properties.${key}: ${words(error)}`);
      }
    }
    set.push(...keys);
  } catch (error) {
    const unrestored = await rollBack(undo);
    throw new Error(`${words(error)}. Nothing was changed${unrestored ? `, except what could not be put back: ${unrestored}` : ""}`);
  }
  return { id: node.id, set };
}

function words(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function rollBack(undo: Undo[]): Promise<string | undefined> {
  const failed: string[] = [];
  for (const step of undo.reverse()) {
    try {
      await step();
    } catch (error) {
      failed.push(words(error));
    }
  }
  return failed.length ? failed.join("; ") : undefined;
}

/** Assign `value` to `key`, keeping what was there. */
function assign(node: Node, key: string, value: unknown, undo: Undo[]): void {
  if (!(key in node)) throw new Error(`a ${node.type} has no ${key}`);
  const before = node[key];
  node[key] = value;
  undo.push(() => {
    node[key] = before;
  });
}

/** Bind `fields` of `node` to a variable, keeping the bindings there. */
function bind(node: Node, fields: readonly string[], variable: Variable, undo: Undo[]): void {
  const target = node as unknown as SceneNode & { setBoundVariable(field: string, variable: Variable | null): void; boundVariables?: Record<string, VariableAlias> };
  for (const field of fields) {
    const before = target.boundVariables?.[field];
    const value = node[field];
    target.setBoundVariable(field, variable);
    undo.push(async () => {
      target.setBoundVariable(field, before ? await figma.variables.getVariableByIdAsync(before.id) : null);
      if (!before) node[field] = value;
    });
  }
}

const isBinding = (value: unknown): value is { variable: string } =>
  typeof value === "object" && value !== null && typeof (value as { variable?: unknown }).variable === "string";

async function one(node: Node, key: string, value: unknown, undo: Undo[], variables: Variables): Promise<void> {
  if (PLAIN.has(key)) return assign(node, key, value, undo);
  if (key in BINDABLE) {
    const fields = BINDABLE[key];
    if (isBinding(value)) return bind(node, fields, await variables.named(value.variable), undo);
    for (const field of fields) assign(node, field, value, undo);
    return;
  }
  if (key === "fills" || key === "strokes") return assign(node, key, await paintsIn(key, value, (name) => variables.named(name)), undo);
  if (key === "effects") return assign(node, key, effects(value), undo);
  if (key in STYLES) return style(node, key, value, undo);
  if (key === "explicitVariableModes") return modes(node, value, undo);
  if (key === "componentProperties") return componentProperties(node, value, undo);
  throw new Error(`not a property set_properties takes`);
}

/** Effects as set_properties takes them, made into the plugin API's. */
export function effects(given: unknown): Effect[] {
  if (!Array.isArray(given)) throw new Error("expected an array of effects");
  return given.map((item: Record<string, unknown>, index): Effect => {
    const visible = item.visible !== false;
    const radius = Number(item.radius ?? 0);
    if (item.type === "LAYER_BLUR" || item.type === "BACKGROUND_BLUR") return { type: item.type, radius, visible, blurType: "NORMAL" };
    if (item.type !== "DROP_SHADOW" && item.type !== "INNER_SHADOW") {
      throw new Error(`[${index}].type: \`${String(item.type)}\` is not one of DROP_SHADOW, INNER_SHADOW, LAYER_BLUR, BACKGROUND_BLUR`);
    }
    const color = parseHex(`[${index}].color`, item.color ?? "#00000040");
    const alpha = typeof item.opacity === "number" ? item.opacity : color.a;
    const offset = (item.offset ?? { x: 0, y: 0 }) as Vector;
    const spread = Number(item.spread ?? 0);
    return { type: item.type, color: { r: color.r, g: color.g, b: color.b, a: alpha }, offset, radius, spread, visible, blendMode: "NORMAL" };
  });
}

async function style(node: Node, key: string, value: unknown, undo: Undo[]): Promise<void> {
  const { set, id, kind } = STYLES[key];
  if (!(set in node)) throw new Error(`a ${node.type} takes no ${key}`);
  let styleId = "";
  if (value !== null) {
    const wanted = String(value);
    const local =
      kind === "PAINT"
        ? await figma.getLocalPaintStylesAsync()
        : kind === "EFFECT"
          ? await figma.getLocalEffectStylesAsync()
          : await figma.getLocalTextStylesAsync();
    const found = local.find((s) => s.name === wanted || s.id === wanted) ?? (await figma.getStyleByIdAsync(wanted));
    if (!found) throw new Error(`no ${kind.toLowerCase()} style named \`${wanted}\`: list_styles lists them`);
    styleId = found.id;
  }
  const before = read(node, id);
  if (kind === "TEXT") await loadFonts(node as unknown as TextNode);
  const setter = node[set] as (id: string) => Promise<void>;
  await setter.call(node, styleId);
  undo.push(async () => {
    if (typeof before === "string") await setter.call(node, before);
  });
}

async function modes(node: Node, value: unknown, undo: Undo[]): Promise<void> {
  const target = node as unknown as SceneNode & ExplicitVariableModesMixin;
  if (!("setExplicitVariableModeForCollection" in target)) throw new Error(`a ${node.type} takes no explicitVariableModes`);
  const collections = await figma.variables.getLocalVariableCollectionsAsync();
  for (const [name, mode] of Object.entries((value ?? {}) as Record<string, unknown>)) {
    const collection = collections.find((c) => c.name === name || isId(c.id, name));
    if (!collection) throw new Error(`no variable collection named \`${name}\``);
    const before = target.explicitVariableModes[collection.id];
    const restore = () =>
      before ? target.setExplicitVariableModeForCollection(collection, before) : target.clearExplicitVariableModeForCollection(collection);
    if (mode === null) {
      target.clearExplicitVariableModeForCollection(collection);
    } else {
      const found = collection.modes.find((m) => m.name === mode || m.modeId === mode);
      if (!found) throw new Error(`${name} has no mode \`${String(mode)}\`; it has ${collection.modes.map((m) => m.name).join(", ")}`);
      target.setExplicitVariableModeForCollection(collection, found.modeId);
    }
    undo.push(restore);
  }
}

async function size(node: Node, properties: Record<string, unknown>, undo: Undo[], variables: Variables): Promise<void> {
  const target = node as unknown as SceneNode & LayoutMixin;
  if (!("resize" in target)) throw new Error(`properties.width: a ${node.type} has no size`);
  const [width, height] = [properties.width, properties.height];
  const before = { width: target.width, height: target.height };
  try {
    if (node.type === "TEXT") {
      await loadFonts(node as unknown as TextNode);
      // As on a file: a width fixes the width and wraps; a height fixes the box.
      assign(node, "textAutoResize", height !== undefined ? "NONE" : "HEIGHT", undo);
    }
    const number = (value: unknown, now: number) => (typeof value === "number" ? value : now);
    target.resize(number(width, target.width), number(height, target.height));
    undo.push(() => target.resize(before.width, before.height));
    if (isBinding(width)) bind(node, ["width"], await variables.named(width.variable), undo);
    if (isBinding(height)) bind(node, ["height"], await variables.named(height.variable), undo);
  } catch (error) {
    throw new Error(`properties.${width !== undefined ? "width" : "height"}: ${words(error)}`);
  }
}

export async function loadFonts(text: TextNode): Promise<void> {
  const fonts = text.characters.length ? text.getRangeAllFontNames(0, text.characters.length) : [text.fontName as FontName];
  await Promise.all(fonts.map((font) => figma.loadFontAsync(font)));
}

/** A text's keys, its fonts loaded first: the new one too. */
async function textChanges(node: Node, properties: Record<string, unknown>, undo: Undo[], variables: Variables): Promise<void> {
  if (node.type !== "TEXT") {
    const key = Object.keys(properties).find((k) => TEXT.has(k));
    throw new Error(`properties.${key}: ${node.id} is a ${node.type}, not a text`);
  }
  const text = node as unknown as TextNode;
  await loadFonts(text);
  const at = (key: string, step: () => Promise<void> | void) =>
    Promise.resolve()
      .then(step)
      .catch((error: unknown) => {
        throw new Error(`properties.${key}: ${words(error)}`);
      });
  if ("fontFamily" in properties || "fontStyle" in properties) {
    // The font in use, from the fonts the text's characters hold, as loadFonts
    // read them: on a text no plugin has set yet, `fontName` was seen giving
    // no family (2026-11-22), and "undefined Bold" was asked of Figma.
    const held = text.characters.length ? text.getRangeAllFontNames(0, text.characters.length) : [];
    const now = held[0] ?? (text.fontName === figma.mixed ? text.getRangeFontName(0, 1) : text.fontName) as FontName;
    const family = properties.fontFamily ?? now?.family;
    const style = properties.fontStyle ?? now?.style;
    if (typeof family !== "string" || typeof style !== "string") {
      throw new Error(`properties.${"fontFamily" in properties ? "fontStyle" : "fontFamily"}: Figma gave this text no font to keep; pass fontFamily and fontStyle together`);
    }
    const font = { family, style };
    await at("fontFamily" in properties ? "fontFamily" : "fontStyle", async () => {
      await figma.loadFontAsync(font);
      assign(node, "fontName", font, undo);
    });
  }
  for (const key of Object.keys(properties)) {
    if (!TEXT.has(key) || key === "fontFamily" || key === "fontStyle") continue;
    const value = properties[key];
    await at(key, async () => {
      if (key === "fontSize" && isBinding(value)) return bind(node, ["fontSize"], await variables.named(value.variable), undo);
      if (key === "lineHeight" || key === "letterSpacing") {
        const given = typeof value === "number" ? { value, unit: "PIXELS" } : value;
        return assign(node, key, given, undo);
      }
      assign(node, key, value, undo);
    });
  }
}

/**
 * Runs, replacing the characters and their styles: each new character takes
 * the layer's style, as the first character holds it once the layer's own keys
 * are set, and a run's keys over it. Then paragraphs, one a line.
 */
async function richText(node: Node, properties: Record<string, unknown>, undo: Undo[], variables: Variables): Promise<void> {
  const key = "runs" in properties ? "runs" : "paragraphs";
  if (node.type !== "TEXT") throw new Error(`properties.${key}: ${node.id} is a ${node.type}, not a text`);
  const text = node as unknown as TextNode;
  const runs = "runs" in properties ? runsIn(properties.runs) : undefined;
  const characters = runs ? runs.map((run) => run.characters).join("") : text.characters;
  const paragraphs = "paragraphs" in properties ? paragraphsIn(characters, properties.paragraphs) : undefined;
  await loadFonts(text);
  undo.push(kept(text));
  if (runs) {
    const layer = await replaced(text, characters);
    await styleRuns(text, runs, layer, (name) => variables.named(name));
  }
  if (paragraphs) setParagraphs(text, paragraphs);
}

/** An instance's values for its component's properties, by name, as on a file. */
async function componentProperties(node: Node, value: unknown, undo: Undo[]): Promise<void> {
  if (node.type !== "INSTANCE") throw new Error(`${node.id} is not an instance`);
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`expected { "<property>": value }`);
  const instance = node as unknown as InstanceNode;
  const held = instance.componentProperties;
  const names = Object.keys(held).filter((k) => held[k].type !== "VARIANT").map(nameOf);
  const wanted: Record<string, string | boolean> = {};
  const before: Record<string, string | boolean> = {};
  for (const [name, raw] of Object.entries(value as Record<string, unknown>)) {
    const at = `componentProperties.${name}`;
    const keys = Object.keys(held).filter((k) => k === name || nameOf(k) === name);
    if (keys.length !== 1) {
      throw new Error(
        keys.length
          ? `${at}: ${keys.length} properties are named \`${name}\`; pass one key: ${keys.join(", ")}`
          : `${at}: no property \`${name}\` on this instance's component; it has ${names.join(", ") || "none"}`,
      );
    }
    const [key] = keys;
    const { type } = held[key];
    if (type === "VARIANT") throw new Error(`${at}: a VARIANT property is not set here; variants are set with set_variant`);
    if (type === "TEXT") {
      if (typeof raw !== "string" && typeof raw !== "number") throw new Error(`${at}: a TEXT property takes a string, got ${JSON.stringify(raw)}`);
      wanted[key] = String(raw);
    } else if (type === "BOOLEAN") {
      if (typeof raw !== "boolean") throw new Error(`${at}: a BOOLEAN property takes true or false, got ${JSON.stringify(raw)}`);
      wanted[key] = raw;
    } else {
      wanted[key] = await swapTarget(String(raw)).then(
        (component) => component.id,
        (error: unknown) => {
          throw new Error(`${at}: ${words(error)}`);
        },
      );
    }
    before[key] = held[key].value;
  }
  instance.setProperties(wanted);
  undo.push(() => instance.setProperties(before));
}

/** A property's name, from its key: `Label#12:3` is `Label`. */
function nameOf(key: string): string {
  const hash = key.lastIndexOf("#");
  return hash > 0 ? key.slice(0, hash) : key;
}

/** The file's own variables, found by name or id. */
export class Variables {
  private local?: Variable[];

  async named(wanted: string): Promise<Variable> {
    this.local ??= await figma.variables.getLocalVariablesAsync();
    const found = this.local.filter((v) => v.name === wanted || isId(v.id, wanted));
    if (found.length > 1) throw new Error(`${found.length} variables are named \`${wanted}\`; pass one id: ${found.map((v) => bare(v.id)).join(", ")}`);
    if (!found.length) throw new Error(`no local variable named \`${wanted}\`: list_variables lists them`);
    return found[0];
  }
}
