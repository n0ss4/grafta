// Layers the person keeps from the AI: no call changes them or anything in
// them, and the refusal says why. Kept in the document's plugin data, so the
// file remembers them for whoever opens the panel on it; Figma keeps plugin
// data apart from the design, and nothing of it is drawn.

const KEY = "grafta-protected";

export interface Protected {
  id: string;
  name: string;
  type: string;
}

export function list(): Protected[] {
  try {
    const kept: unknown = JSON.parse(figma.root.getPluginData(KEY) || "[]");
    return Array.isArray(kept) ? kept.filter((p): p is Protected => typeof p?.id === "string") : [];
  } catch {
    return [];
  }
}

function save(all: Protected[]): void {
  figma.root.setPluginData(KEY, JSON.stringify(all));
}

/** Protect what the person has selected, or the page when nothing is. */
export function protectSelection(): Protected[] {
  const chosen: BaseNode[] = figma.currentPage.selection.length ? [...figma.currentPage.selection] : [figma.currentPage];
  const all = list();
  for (const node of chosen) {
    if (!all.some((p) => p.id === node.id)) all.push({ id: node.id, name: node.name, type: node.type });
  }
  save(all);
  return all;
}

export function unprotect(id: string): Protected[] {
  const all = list().filter((p) => p.id !== id);
  save(all);
  return all;
}

/** The arguments of a call that name the layers it writes to or into. */
const WRITES_TO = ["id", "parent", "also", "ids", "behind", "in_front_of", "component"];

/** Why a call may not run, when it would change a protected layer; nothing when it may. */
export async function refusal(args: Record<string, unknown>): Promise<string | undefined> {
  const all = list();
  if (!all.length) return undefined;
  const guarded = new Map(all.map((p) => [p.id, p]));
  const named = WRITES_TO.flatMap((key) => {
    const value = args[key];
    return typeof value === "string" ? [value] : Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];
  });
  for (const id of named) {
    for (let node = await figma.getNodeByIdAsync(id).catch(() => null); node; node = node.parent) {
      const hit = guarded.get(node.id);
      if (hit) {
        const inside = node.id === id ? "" : ", which holds it,";
        return `“${hit.name}”${inside} is protected in grafta's panel in Figma: nothing was changed. The person can unprotect it there`;
      }
    }
  }
  return undefined;
}
