// A layer as it was before a request first touched it, kept to put it back:
// "Revert this request" in the panel. What a property is set to through the
// plugin API is read back here the same way, so it can be assigned again.
// A layer the request made is removed; one it deleted cannot come back, and
// the panel says so.

/** The properties put back, in the plugin API's names, where the layer has them. */
const FIELDS = [
  "name", "visible", "locked", "opacity", "blendMode", "isMask", "rotation",
  "fills", "strokes", "strokeWeight", "strokeAlign", "strokeCap", "strokeJoin", "dashPattern", "effects",
  "cornerRadius", "topLeftRadius", "topRightRadius", "bottomLeftRadius", "bottomRightRadius", "clipsContent",
  "layoutMode", "layoutWrap", "itemSpacing", "counterAxisSpacing", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
  "primaryAxisSizingMode", "counterAxisSizingMode", "primaryAxisAlignItems", "counterAxisAlignItems",
  "layoutGrow", "layoutAlign", "layoutPositioning", "constraints",
  "characters", "fontName", "fontSize", "lineHeight", "letterSpacing", "textAlignHorizontal", "textAlignVertical",
  "textAutoResize", "textDecoration", "textCase", "paragraphSpacing", "paragraphIndent",
] as const;

export interface Kept {
  id: string;
  fields: Record<string, unknown>;
  parent?: string;
  index?: number;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}

/** The layer's properties now, those that are one value (a mixed one is not kept). */
export function keep(node: BaseNode): Kept {
  const kept: Kept = { id: node.id, fields: {} };
  const any = node as unknown as Record<string, unknown>;
  for (const field of FIELDS) {
    if (!(field in node)) continue;
    const value = any[field];
    if (value === figma.mixed || typeof value === "function") continue;
    kept.fields[field] = value;
  }
  if (node.parent) {
    kept.parent = node.parent.id;
    kept.index = node.parent.children.indexOf(node as SceneNode);
  }
  if ("x" in node) {
    const scene = node as SceneNode;
    kept.x = scene.x;
    kept.y = scene.y;
    kept.width = scene.width;
    kept.height = scene.height;
  }
  return kept;
}

/** Put the layer back as kept. Returns what could not be, in the person's words. */
export async function putBack(kept: Kept): Promise<string | undefined> {
  const node = await figma.getNodeByIdAsync(kept.id).catch(() => null);
  if (!node || node.removed) return "it is no longer in the file";
  const any = node as unknown as Record<string, unknown>;
  const differs = (field: string) => field in kept.fields && JSON.stringify(any[field]) !== JSON.stringify(kept.fields[field]);
  if (node.type === "TEXT") {
    const text = node as TextNode;
    const fonts = text.characters.length ? text.getRangeAllFontNames(0, text.characters.length) : [];
    const old = kept.fields.fontName as FontName | undefined;
    await Promise.all([...fonts, ...(old ? [old] : [])].map((font) => figma.loadFontAsync(font)));
    // The font first: the characters put back take it.
    if (differs("fontName")) any.fontName = kept.fields.fontName;
  }
  const failed: string[] = [];
  // Size and place first: resize() on an auto-layout frame fixes its sizing,
  // so the sizing modes kept are assigned after it, and a hugging frame hugs again.
  if (kept.parent && node.parent?.id !== kept.parent) {
    const parent = await figma.getNodeByIdAsync(kept.parent).catch(() => null);
    if (parent && "insertChild" in parent) {
      const at = Math.min(kept.index ?? (parent as ChildrenMixin).children.length, (parent as ChildrenMixin).children.length);
      (parent as ChildrenMixin).insertChild(at, node as SceneNode);
    } else failed.push("its place");
  } else if (kept.parent && node.parent && kept.index !== undefined) {
    const siblings = (node.parent as ChildrenMixin).children;
    if (siblings.indexOf(node as SceneNode) !== kept.index && kept.index < siblings.length) {
      (node.parent as ChildrenMixin).insertChild(kept.index, node as SceneNode);
    }
  }
  if ("x" in node && kept.width !== undefined && kept.height !== undefined) {
    const scene = node as SceneNode & LayoutMixin;
    const fixed = !("textAutoResize" in node) || (node as TextNode).textAutoResize === "NONE";
    if (fixed && (Math.abs(scene.width - kept.width) > 0.01 || Math.abs(scene.height - kept.height) > 0.01) && "resize" in scene) {
      try {
        scene.resize(kept.width, kept.height);
      } catch {
        failed.push("its size");
      }
    }
    if (kept.x !== undefined && kept.y !== undefined && (scene.x !== kept.x || scene.y !== kept.y)) {
      scene.x = kept.x;
      scene.y = kept.y;
    }
  }
  // The properties last, sizing modes among them.
  for (const [field, value] of Object.entries(kept.fields)) {
    if (field === "fontName" || !differs(field)) continue;
    try {
      any[field] = value;
    } catch {
      failed.push(field);
    }
  }
  return failed.length ? `${failed.join(", ")} could not be put back` : undefined;
}

/** Remove a layer the request made. */
export async function unmake(id: string): Promise<string | undefined> {
  const node = await figma.getNodeByIdAsync(id).catch(() => null);
  if (!node || node.removed) return undefined;
  if (node.type === "DOCUMENT") return "the file cannot be removed";
  node.remove();
  return undefined;
}
