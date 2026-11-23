// A node in Figma's terms, as read_node shows it on a file: the rules of
// grafta's crates/core/app/src/view.rs, rule for rule. A property at Figma's
// default is left out, and absence means exactly that.
import { hex, paintsOut } from "./paint";
import { Names, num, read } from "./nodes";
import { bytes } from "./paging";

/** How deep and how big a view may get. */
export interface Limits {
  depth: number;
  budget: number;
  geometry: boolean;
  childrenFrom: number;
}

/** The most nodes one read walks, whatever its depth and budget: research R4. */
export const NODE_CAP = 5000;

type View = Record<string, unknown>;

/** What a walk has spent: bytes as JSON, and nodes. */
interface Spent {
  bytes: number;
  nodes: number;
  capped: boolean;
}


/** A node's view with its children to `limits.depth`, cut at a node boundary. */
export async function view(node: BaseNode, limits: Limits, names: Names): Promise<View> {
  const inner = { ...limits, budget: limits.budget - Math.floor(limits.budget / 8) };
  const spent: Spent = { bytes: 0, nodes: 0, capped: false };
  const out = await subtree(node, limits.depth, limits.childrenFrom, inner, names, spent);
  const references = await propertyReferences(node);
  if (references) out.componentPropertyReferences = references;
  const modes = await explicitModes(node, names);
  if (modes) out.explicitVariableModes = modes;
  const reactions = read(node, "reactions");
  if (Array.isArray(reactions) && reactions.length) out.reactions = reactions;
  if (node.type === "PAGE" && node.flowStartingPoints.length) out.flowStartingPoints = node.flowStartingPoints;
  return out;
}

async function subtree(node: BaseNode, depth: number, skip: number, limits: Limits, names: Names, spent: Spent): Promise<View> {
  const out = await properties(node, limits.geometry, names);
  spent.bytes += bytes(out) + 48;
  spent.nodes += 1;
  // A page of Figma's is loaded before its children are read, even to count them.
  if (node.type === "PAGE") await node.loadAsync();
  const children = "children" in node ? (node as ChildrenMixin).children : [];
  if (children.length === 0) return out;
  out.childCount = children.length;
  if (depth === 0) {
    out.childrenOmitted = true;
    return out;
  }
  const listed: View[] = [];
  let cutAt: number | undefined;
  let reason = "budget";
  for (let position = skip; position < children.length; position++) {
    if (spent.nodes >= NODE_CAP && listed.length) {
      [cutAt, reason, spent.capped] = [position, "nodes", true];
      break;
    }
    const before = spent.bytes;
    const child = await subtree(children[position], depth - 1, 0, limits, names, spent);
    if (spent.bytes > limits.budget && listed.length) {
      spent.bytes = before;
      cutAt = position;
      break;
    }
    listed.push(child);
  }
  out.children = listed;
  if (skip > 0) out.childrenFrom = skip;
  if (cutAt !== undefined) {
    out.omitted = {
      reason,
      remaining_children: children.length - cutAt,
      resume: { id: node.id, children_from: cutAt, depth },
      how: "call read_node with this id, depth and children_from",
    };
  }
  return out;
}

const DEFAULT_BLEND = ["PASS_THROUGH", "NORMAL"];

/** One node's own properties, defaults left out. */
export async function properties(node: BaseNode, geometry: boolean, names: Names): Promise<View> {
  const v: View = { id: node.id, type: node.type, name: node.name };
  const scene = node.type !== "DOCUMENT" && node.type !== "PAGE" ? (node as SceneNode) : undefined;
  if (scene && "x" in scene) {
    v.x = num(scene.x);
    v.y = num(scene.y);
    v.width = num(scene.width);
    v.height = num(scene.height);
  }
  if (node.type === "ELLIPSE") {
    const arc = node.arcData;
    const full = Math.abs(arc.endingAngle - arc.startingAngle - 2 * Math.PI) < 1e-4 && arc.innerRadius === 0;
    if (!full) v.arcData = { startingAngle: num(arc.startingAngle), endingAngle: num(arc.endingAngle), innerRadius: num(arc.innerRadius) };
  }
  if (node.type === "POLYGON" || node.type === "STAR") v.pointCount = node.pointCount;
  if (node.type === "STAR") v.innerRadius = num(node.innerRadius);
  const rotation = read(node, "rotation");
  if (typeof rotation === "number" && rotation !== 0) v.rotation = num(rotation);
  if (scene && !scene.visible) v.visible = false;
  if (scene && scene.locked) v.locked = true;
  const opacity = read(node, "opacity");
  if (typeof opacity === "number" && opacity < 1) v.opacity = num(opacity);
  const blend = read(node, "blendMode");
  if (typeof blend === "string" && !DEFAULT_BLEND.includes(blend)) v.blendMode = blend;

  const fills = read(node, "fills");
  if (Array.isArray(fills) && fills.length) v.fills = await paintsOut(fills, names);
  const fillStyle = await names.style(read(node, "fillStyleId"));
  if (fillStyle) v.fillStyle = fillStyle;
  const strokes = read(node, "strokes");
  if (Array.isArray(strokes) && strokes.length) {
    const s = node as MinimalStrokesMixin & Partial<IndividualStrokesMixin>;
    v.strokes = await paintsOut(strokes, names);
    v.strokeWeight = typeof s.strokeWeight === "number" ? num(s.strokeWeight) : "MIXED";
    v.strokeAlign = s.strokeAlign;
    if (s.strokeWeight === figma.mixed && s.strokeTopWeight !== undefined) {
      v.strokeWeights = { top: num(s.strokeTopWeight), right: num(s.strokeRightWeight ?? 0), bottom: num(s.strokeBottomWeight ?? 0), left: num(s.strokeLeftWeight ?? 0) };
    }
    if (s.dashPattern.length) v.dashPattern = s.dashPattern.map(num);
  }
  const strokeStyle = await names.style(read(node, "strokeStyleId"));
  if (strokeStyle) v.strokeStyle = strokeStyle;
  const effects = read(node, "effects");
  if (Array.isArray(effects) && effects.length) v.effects = effects.map(effect);
  const effectStyle = await names.style(read(node, "effectStyleId"));
  if (effectStyle) v.effectStyle = effectStyle;
  radii(node, v);
  if (read(node, "clipsContent") === true) v.clipsContent = true;
  if (read(node, "isMask") === true) v.isMask = true;

  layout(node, v);
  sizing(node, v);
  const constraints = read(node, "constraints") as Constraints | undefined;
  if (constraints && (constraints.horizontal !== "MIN" || constraints.vertical !== "MIN")) v.constraints = constraints;
  for (const key of ["minWidth", "maxWidth", "minHeight", "maxHeight"]) {
    const value = read(node, key);
    if (typeof value === "number") v[key] = num(value);
  }
  if (node.type === "TEXT") await text(node, v, names);

  const bound = read(node, "boundVariables") as Record<string, unknown> | undefined;
  if (bound) {
    const others: View = {};
    for (const [field, alias] of Object.entries(bound)) {
      if (field === "fills" || field === "strokes" || alias === undefined || Array.isArray(alias)) continue;
      const label = await names.label(alias);
      if (label) others[field] = label;
    }
    if (Object.keys(others).length) v.boundVariables = others;
  }
  await component(node, v);
  if (geometry) {
    const fillGeometry = read(node, "fillGeometry");
    const strokeGeometry = read(node, "strokeGeometry");
    if (Array.isArray(fillGeometry) && (fillGeometry.length || (Array.isArray(strokeGeometry) && strokeGeometry.length))) {
      const paths = (list: unknown) => (Array.isArray(list) ? list.map((p: VectorPath) => ({ d: p.data, windingRule: p.windingRule })) : []);
      v.fillGeometry = paths(fillGeometry);
      v.strokeGeometry = paths(strokeGeometry);
    }
  } else {
    const fillPaths = read(node, "fillGeometry");
    const strokePaths = read(node, "strokeGeometry");
    const counts = [fillPaths, strokePaths].map((list) => (Array.isArray(list) ? list.length : 0));
    if ((node.type === "VECTOR" || node.type === "BOOLEAN_OPERATION" || node.type === "STAR" || node.type === "POLYGON") && counts[0] + counts[1] > 0) {
      v.geometry = { fillPaths: counts[0], strokePaths: counts[1], how: "read_node with geometry: true" };
    }
  }
  const exports = read(node, "exportSettings");
  if (Array.isArray(exports) && exports.length) v.exportSettings = exports;
  return v;
}

/** An effect as read_node shows it. */
export function effect(e: Effect): View {
  const out: View = { type: e.type };
  if ("radius" in e) out.radius = num(e.radius);
  if ("color" in e) {
    out.color = hex(e.color);
    if (e.color.a < 1) out.opacity = num(e.color.a);
  }
  if ("offset" in e && (e.offset.x !== 0 || e.offset.y !== 0)) out.offset = { x: num(e.offset.x), y: num(e.offset.y) };
  if ("spread" in e && e.spread) out.spread = num(e.spread);
  if (!e.visible) out.visible = false;
  return out;
}

function radii(node: BaseNode, v: View): void {
  const corners = ["topLeftRadius", "topRightRadius", "bottomRightRadius", "bottomLeftRadius"].map((key) => read(node, key));
  if (corners.every((c) => typeof c === "number")) {
    const [tl, tr, br, bl] = corners as number[];
    if (tl === 0 && tr === 0 && br === 0 && bl === 0) return;
    if (tl === tr && tr === br && br === bl) v.cornerRadius = num(tl);
    else Object.assign(v, { topLeftRadius: num(tl), topRightRadius: num(tr), bottomRightRadius: num(br), bottomLeftRadius: num(bl) });
    return;
  }
  const radius = read(node, "cornerRadius");
  if (typeof radius === "number" && radius !== 0) v.cornerRadius = num(radius);
}

function inAutoLayout(node: BaseNode): boolean {
  const parent = node.parent;
  return !!parent && "layoutMode" in parent && (parent as FrameNode).layoutMode !== "NONE";
}

function layout(node: BaseNode, v: View): void {
  if ("layoutMode" in node && node.layoutMode !== "NONE") {
    const f = node as FrameNode;
    v.layoutMode = f.layoutMode;
    v.itemSpacing = num(f.itemSpacing);
    if (f.counterAxisSpacing) v.counterAxisSpacing = num(f.counterAxisSpacing);
    for (const key of ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"] as const) if (f[key]) v[key] = num(f[key]);
    v.primaryAxisSizingMode = f.primaryAxisSizingMode;
    v.counterAxisSizingMode = f.counterAxisSizingMode;
    v.primaryAxisAlignItems = f.primaryAxisAlignItems;
    v.counterAxisAlignItems = f.counterAxisAlignItems;
    if (f.layoutWrap === "WRAP") v.layoutWrap = "WRAP";
    if (f.itemReverseZIndex) v.itemReverseZIndex = true;
  }
  if (inAutoLayout(node)) {
    const child = node as SceneNode & AutoLayoutChildrenMixin;
    if (child.layoutPositioning === "ABSOLUTE") v.layoutPositioning = "ABSOLUTE";
    if (child.layoutGrow) v.layoutGrow = num(child.layoutGrow);
    if (child.layoutAlign && child.layoutAlign !== "INHERIT") v.layoutAlign = child.layoutAlign;
  }
}

function sizing(node: BaseNode, v: View): void {
  const own = "layoutMode" in node && (node as FrameNode).layoutMode !== "NONE";
  if (!own && !inAutoLayout(node)) return;
  for (const key of ["layoutSizingHorizontal", "layoutSizingVertical"]) {
    const value = read(node, key);
    if (typeof value === "string") v[key] = value;
  }
}

const first = <T>(value: T | PluginAPI["mixed"], fallback: () => T): T => (value === figma.mixed ? fallback() : (value as T));

async function text(node: TextNode, v: View, names: Names): Promise<void> {
  v.characters = node.characters;
  const segments = node.characters.length
    ? node.getStyledTextSegments([
        "fontName",
        "fontSize",
        "fontWeight",
        "fills",
        "textDecoration",
        "textCase",
        "letterSpacing",
        "lineHeight",
        "hyperlink",
        "textStyleId",
        "listOptions",
        "indentation",
      ])
    : [];
  const base = segments[0];
  const font = first(node.fontName, () => base?.fontName ?? { family: "", style: "" });
  const style: View = {
    fontFamily: font.family,
    fontStyle: font.style,
    fontSize: num(first(node.fontSize, () => base?.fontSize ?? 0)),
    fontWeight: first(node.fontWeight, () => base?.fontWeight ?? 400),
  };
  const lineHeight = first(node.lineHeight, () => base?.lineHeight ?? { unit: "AUTO" });
  if (lineHeight.unit !== "AUTO") style.lineHeight = { value: num(lineHeight.value), unit: lineHeight.unit };
  const spacing = first(node.letterSpacing, () => base?.letterSpacing ?? { value: 0, unit: "PERCENT" });
  if (spacing.value !== 0) style.letterSpacing = { value: num(spacing.value), unit: spacing.unit };
  style.textAlignHorizontal = node.textAlignHorizontal;
  style.textAlignVertical = node.textAlignVertical;
  const textCase = first(node.textCase, () => base?.textCase ?? "ORIGINAL");
  if (textCase !== "ORIGINAL") style.textCase = textCase;
  const decoration = first(node.textDecoration, () => base?.textDecoration ?? "NONE");
  if (decoration !== "NONE") style.textDecoration = decoration;
  if (node.textAutoResize !== "NONE") style.textAutoResize = node.textAutoResize;
  if (node.maxLines !== null) style.maxLines = node.maxLines;
  const paragraphSpacing = first(node.paragraphSpacing, () => 0);
  if (paragraphSpacing) style.paragraphSpacing = num(paragraphSpacing);
  const paragraphIndent = first(node.paragraphIndent, () => 0);
  if (paragraphIndent) style.paragraphIndent = num(paragraphIndent);
  v.style = style;
  if (segments.length > 1) v.runs = await runs(segments, names);
  if (segments.some((s) => s.listOptions.type !== "NONE")) {
    let start = 0;
    v.paragraphs = node.characters.split("\n").map((line) => {
      const at = segments.find((s) => s.start <= start && start < s.end) ?? base;
      start += line.length + 1;
      return at.listOptions.type === "NONE" ? { listType: "NONE" } : { listType: at.listOptions.type, indentation: at.indentation };
    });
  }
  const textStyle = await names.style(first(node.textStyleId, () => ""));
  if (textStyle) v.textStyle = textStyle;
}

type Segment = Pick<
  StyledTextSegment,
  "characters" | "start" | "end" | "fontName" | "fontSize" | "fills" | "textDecoration" | "textCase" | "letterSpacing" | "lineHeight" | "hyperlink" | "textStyleId"
>;

/** A text's runs in create_text's names: each run's characters, and what it sets. */
async function runs(segments: Segment[], names: Names): Promise<unknown[]> {
  const out: unknown[] = [];
  for (const s of segments) {
    const r: View = { characters: s.characters, fontFamily: s.fontName.family, fontStyle: s.fontName.style, fontSize: num(s.fontSize) };
    r.fills = await paintsOut(s.fills, names);
    if (s.textDecoration !== "NONE") r.textDecoration = s.textDecoration;
    if (s.textCase !== "ORIGINAL") r.textCase = s.textCase;
    if (s.letterSpacing.value !== 0) r.letterSpacing = { value: num(s.letterSpacing.value), unit: s.letterSpacing.unit };
    if (s.lineHeight.unit !== "AUTO") r.lineHeight = { value: num(s.lineHeight.value), unit: s.lineHeight.unit };
    if (s.hyperlink?.type === "URL") r.hyperlink = { type: "URL", value: s.hyperlink.value };
    const style = await names.style(s.textStyleId);
    if (style) r.textStyle = style;
    out.push(r);
  }
  return out;
}

async function component(node: BaseNode, v: View): Promise<void> {
  if (node.type === "INSTANCE") {
    const main = await node.getMainComponentAsync();
    if (main) {
      v.componentId = main.id;
      v.componentName = main.parent?.type === "COMPONENT_SET" ? main.parent.name : main.name;
      if (main.remote) v.componentNotInFile = "the definition lives in another file, a published library";
      if (main.remote && main.key) v.componentKey = main.key;
      const variant = node.variantProperties;
      if (variant && Object.keys(variant).length) v.variant = variant;
    }
    const values: View = {};
    for (const [key, property] of Object.entries(node.componentProperties)) {
      if (property.type === "VARIANT") continue;
      let shown: unknown = property.value;
      if (property.type === "INSTANCE_SWAP" && typeof property.value === "string") {
        shown = (await figma.getNodeByIdAsync(property.value))?.name ?? property.value;
      }
      values[key.split("#")[0]] = shown;
    }
    if (Object.keys(values).length) v.componentProperties = values;
    const overrides = node.overrides.map((o) => ({ id: o.id, fields: o.overriddenFields }));
    if (overrides.length) v.overrides = overrides;
  }
  if (node.type === "COMPONENT") {
    const variant = node.variantProperties;
    if (variant && Object.keys(variant).length) v.variantProperties = variant;
  }
  if (node.type === "COMPONENT_SET") {
    const options = Object.fromEntries(Object.entries(node.variantGroupProperties).map(([key, group]) => [key, group.values]));
    if (Object.keys(options).length) v.variantOptions = options;
  }
  if ((node.type === "COMPONENT" && node.parent?.type !== "COMPONENT_SET") || node.type === "COMPONENT_SET") {
    const definitions = Object.entries(node.componentPropertyDefinitions)
      .filter(([, d]) => d.type !== "VARIANT")
      .map(([key, d]) => {
        const out: View = { id: key, name: key.split("#")[0], type: d.type };
        if (d.defaultValue !== undefined) out.default = d.defaultValue;
        if (d.variantOptions?.length) out.options = d.variantOptions;
        return out;
      });
    if (definitions.length) v.componentPropertyDefinitions = definitions;
  }
  if ((node.type === "COMPONENT" || node.type === "COMPONENT_SET") && node.description) v.description = node.description;
}

/** A layer's bindings to its component's properties, by property name. */
async function propertyReferences(node: BaseNode): Promise<View | undefined> {
  const refs = read(node, "componentPropertyReferences") as Record<string, string> | null | undefined;
  if (!refs || !Object.keys(refs).length) return undefined;
  const out: View = {};
  for (const [field, key] of Object.entries(refs)) out[field] = key.split("#")[0];
  return out;
}

/** A node's own modes, `{ collection: mode }` by name. */
async function explicitModes(node: BaseNode, names: Names): Promise<View | undefined> {
  const modes = read(node, "explicitVariableModes") as Record<string, string> | undefined;
  if (!modes || !Object.keys(modes).length) return undefined;
  const out: View = {};
  for (const [collectionId, modeId] of Object.entries(modes)) {
    const collection = await names.collection(collectionId);
    const mode = collection?.modes.find((m) => m.modeId === modeId);
    out[collection?.name ?? collectionId] = mode?.name ?? modeId;
  }
  return out;
}
