// The calls of one request, as the person sees them: what a model did for
// one thing they asked, gathered from its calls, which come one after another
// with no word of where a request ends. A pause of GAP_MS ends one.
// For each request: the layers it made, changed and removed, each change
// before and after, a picture of the frame it worked in before and after, and
// what is kept to put it all back.
import { changes as diff } from "./changes";
import type { Change, LayerView, RequestView } from "./protocol";
import { keep, putBack, unmake, type Kept } from "./restore";
import { Names } from "./tools/nodes";
import { view } from "./tools/view";

/** A pause longer than this between two calls starts another request. */
const GAP_MS = 6000;
/** How many requests the panel keeps, and how many the file's history. */
const KEPT = 30;
/** The largest picture of a frame, in the panel's pixels at 2×. */
const THUMB = { width: 280, height: 180 };

type View = Record<string, unknown>;

interface Layer {
  id: string;
  name: string;
  made: boolean;
  removed: boolean;
  base?: View;
  kept?: Kept;
  changes: Change[];
  more: number;
}

export interface Request {
  id: number;
  started: number;
  ended: number;
  caller: string;
  layers: Map<string, Layer>;
  order: string[];
  context?: { id: string; name: string; before?: Uint8Array; after?: Uint8Array };
  reverted: boolean;
  /** What putting it back could not do, or will not. */
  notes: string[];
  /** The rest of this request runs without asking (Ask me first). */
  approved: boolean;
}

const requests: Request[] = [];
let next = 1;

/** The request a call belongs to: the last one, unless it ended or was reverted. */
export function requestFor(caller: string): Request {
  const last = requests[requests.length - 1];
  if (last && !last.reverted && Date.now() - last.ended < GAP_MS && last.caller === caller) return last;
  const fresh: Request = { id: next++, started: Date.now(), ended: Date.now(), caller, layers: new Map(), order: [], reverted: false, notes: [], approved: false };
  requests.push(fresh);
  if (requests.length > KEPT) requests.shift();
  return fresh;
}

export function approveRest(request: Request): void {
  request.approved = true;
}

export function approved(request: Request): boolean {
  return request.approved;
}

/** The layers a call writes to, as its arguments name them. */
function written(args: Record<string, unknown>): string[] {
  const ids: string[] = [];
  for (const key of ["id", "ids", "also"]) {
    const value = args[key];
    if (typeof value === "string") ids.push(value);
    else if (Array.isArray(value)) ids.push(...value.filter((v): v is string => typeof v === "string"));
  }
  return ids;
}

async function node(id: string): Promise<BaseNode | null> {
  return figma.getNodeByIdAsync(id).catch(() => null);
}

async function read(n: BaseNode): Promise<View> {
  return view(n, { depth: 0, budget: 1 << 20, geometry: false, childrenFrom: 0 }, new Names());
}

/** The frame a layer is seen in: itself when it is one, else the nearest holding it. */
function contextOf(n: BaseNode): BaseNode | undefined {
  const framing = new Set(["FRAME", "COMPONENT", "COMPONENT_SET", "INSTANCE", "SECTION", "GROUP"]);
  for (let at: BaseNode | null = n; at; at = at.parent) {
    if (at.type === "PAGE" || at.type === "DOCUMENT") return undefined;
    if (framing.has(at.type)) return at;
  }
  return undefined;
}

/** A small picture of a layer, as Figma draws it; nothing when it cannot be drawn. */
export async function picture(id: string): Promise<Uint8Array | undefined> {
  const n = await node(id);
  if (!n || n.removed || !("exportAsync" in n) || !("width" in n)) return undefined;
  const scene = n as SceneNode;
  const scale = Math.min(2, THUMB.width / Math.max(scene.width, 1), THUMB.height / Math.max(scene.height, 1));
  if (!(scale > 0)) return undefined;
  try {
    return await scene.exportAsync({ format: "PNG", constraint: { type: "SCALE", value: scale } });
  } catch {
    return undefined;
  }
}

/** Before a writing call runs: keep each layer it writes to, the first time, and the frame's picture. */
export async function before(request: Request, args: Record<string, unknown>): Promise<void> {
  for (const id of written(args)) {
    if (request.layers.has(id)) continue;
    const n = await node(id);
    if (!n) continue;
    request.layers.set(id, { id, name: n.name, made: false, removed: false, base: await read(n), kept: keep(n), changes: [], more: 0 });
    request.order.push(id);
  }
  if (!request.context) {
    const first = written(args)[0] ?? (typeof args.parent === "string" ? args.parent : undefined);
    const at = first ? await node(first) : null;
    const context = at ? contextOf(at) : undefined;
    if (context) request.context = { id: context.id, name: context.name, before: await picture(context.id) };
  }
}

/** After a writing call ran: what it made, changed or removed, and the frame's picture now. */
export async function after(request: Request, kind: string, args: Record<string, unknown>, result: unknown): Promise<void> {
  request.ended = Date.now();
  const made = (result as { id?: unknown } | undefined)?.id;
  if (kind === "make" && typeof made === "string" && !request.layers.has(made)) {
    const n = await node(made);
    if (n) {
      request.layers.set(made, { id: made, name: n.name, made: true, removed: false, changes: [], more: 0 });
      request.order.push(made);
      if (!request.context) {
        const context = contextOf(n) ?? (n.type !== "PAGE" ? n : undefined);
        if (context) request.context = { id: context.id, name: context.name };
      }
    } else if (!request.notes.includes(VARIABLES)) {
      request.notes.push(VARIABLES);
    }
  }
  for (const id of written(args)) {
    const layer = request.layers.get(id);
    if (!layer) continue;
    if (kind === "remove") {
      layer.removed = true;
      continue;
    }
    const n = await node(id);
    if (n) layer.name = n.name;
    if (layer.base) {
      const found = await diff(id, layer.base);
      layer.changes = found.changes;
      layer.more = found.more;
    }
  }
  if (request.context) request.context.after = await picture(request.context.id);
}

const VARIABLES = "Variables and styles it made are not removed by Revert: delete them in Figma";
const REMOVED = "A layer it deleted cannot come back from here: use Edit → Undo in Figma";

/** Put a request back: each layer as it was, and what it made removed. One step of undo. */
export async function revert(id: number): Promise<RequestView | undefined> {
  const request = requests.find((r) => r.id === id);
  if (!request || request.reverted) return request && show(request);
  const problems: string[] = [];
  const layers = request.order.map((key) => request.layers.get(key)).filter((l): l is Layer => Boolean(l));
  // Put back first, then remove what it made: a group it made still holds
  // the layers it grouped until they are put back where they were.
  for (const layer of layers) {
    if (layer.made || layer.removed || !layer.kept) continue;
    const why = await putBack(layer.kept);
    if (why) problems.push(`“${layer.name}”: ${why}`);
  }
  for (const layer of [...layers].reverse()) {
    if (!layer.made || layer.removed) continue;
    const why = await unmake(layer.id);
    if (why) problems.push(`“${layer.name}”: ${why}`);
  }
  figma.commitUndo();
  request.reverted = true;
  request.notes = [...request.notes.filter((n) => n !== REMOVED), ...problems];
  if (request.context) request.context.after = await picture(request.context.id);
  return show(request);
}

/** Every layer a request made or changed that is still in the file. */
export function layersOf(id: number): string[] {
  const request = requests.find((r) => r.id === id);
  return request ? request.order.filter((key) => !request.layers.get(key)?.removed) : [];
}

export function show(request: Request): RequestView {
  const layers: LayerView[] = request.order
    .map((key) => request.layers.get(key))
    .filter((l): l is Layer => Boolean(l))
    .filter((l) => l.made || l.removed || l.changes.length)
    .map((l) => ({ id: l.id, name: l.name, made: l.made, removed: l.removed, changes: l.changes, more: l.more }));
  const removed = layers.filter((l) => l.removed && !l.made).length;
  const notes = [...request.notes];
  if (removed && !request.reverted && !notes.includes(REMOVED)) notes.push(REMOVED);
  return {
    id: request.id,
    started: request.started,
    ended: request.ended,
    caller: request.caller,
    made: layers.filter((l) => l.made && !l.removed).length,
    changed: layers.filter((l) => !l.made && !l.removed).length,
    removed,
    layers,
    context: request.context?.name,
    before: request.context?.before,
    after: request.context?.after,
    reverted: request.reverted,
    revertable: !request.reverted && layers.some((l) => !l.removed),
    notes,
    live: true,
  };
}

/** The history the file keeps for the panel, on this computer: words, no pictures. */
const HISTORY = (file: string) => `history:${file}`;

export async function history(file: string): Promise<RequestView[]> {
  const kept: unknown = await figma.clientStorage.getAsync(HISTORY(file)).catch(() => undefined);
  return Array.isArray(kept) ? (kept as RequestView[]).map((r) => ({ ...r, live: false, revertable: false })) : [];
}

export async function remember(file: string, request: RequestView): Promise<void> {
  const all = await history(file);
  const words = { ...request, before: undefined, after: undefined, live: false };
  const at = all.findIndex((r) => r.started === request.started);
  if (at >= 0) all[at] = words;
  else all.push(words);
  await figma.clientStorage.setAsync(HISTORY(file), all.slice(-KEPT)).catch(() => undefined);
}
