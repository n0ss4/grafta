// Moving, copying, grouping and deleting on the open file, each answered in
// the tool's result keys.
import { Args, nodeFor, num } from "./nodes";

type Parent = BaseNode & ChildrenMixin;

/** How many copied layers a duplicate names, as on a file. */
const SHOWN = 50;

async function sceneNode(id: unknown, what = "id"): Promise<SceneNode> {
  const node = await nodeFor(String(id));
  if (node.type === "PAGE" || node.type === "DOCUMENT") throw new Error(`${what}: ${node.id} is a ${node.type}, not a layer`);
  return node as SceneNode;
}

async function parentFor(id: unknown): Promise<Parent> {
  const node = await nodeFor(String(id));
  if (!("appendChild" in node)) throw new Error(`parent: ${node.id} is a ${node.type}, which holds no children`);
  return node as Parent;
}

/** In an auto-layout frame the position is the layout's, not the call's. */
function flowing(node: SceneNode): string | undefined {
  const parent = node.parent;
  if (!parent || !("layoutMode" in parent) || parent.layoutMode === "NONE") return undefined;
  if ("layoutPositioning" in node && node.layoutPositioning === "ABSOLUTE") return undefined;
  return parent.id;
}

function isInside(node: BaseNode, ancestor: BaseNode): boolean {
  for (let at: BaseNode | null = node; at; at = at.parent) if (at.id === ancestor.id) return true;
  return false;
}

export async function moveNode(args: Args): Promise<unknown> {
  const node = await sceneNode(args.id);
  const sibling = typeof args.behind === "string" ? args.behind : typeof args.in_front_of === "string" ? args.in_front_of : undefined;
  let parent: Parent;
  let index: number;
  if (sibling !== undefined) {
    const other = await sceneNode(sibling, args.behind !== undefined ? "behind" : "in_front_of");
    if (!other.parent) throw new Error(`${other.id} has no parent`);
    parent = other.parent as Parent;
    const without = parent.children.filter((c) => c.id !== node.id);
    const at = without.findIndex((c) => c.id === other.id);
    index = args.behind !== undefined ? at : at + 1;
  } else {
    parent = await parentFor(args.parent);
    const others = parent.children.filter((c) => c.id !== node.id).length;
    index = args.to === "back" ? 0 : others;
  }
  if (isInside(parent, node)) throw new Error(`${node.id} cannot go inside itself (${parent.id})`);
  const before = { x: node.x, y: node.y };
  parent.insertChild(index, node);
  node.x = typeof args.x === "number" ? args.x : before.x;
  node.y = typeof args.y === "number" ? args.y : before.y;
  const out: Record<string, unknown> = {
    id: node.id,
    parent: parent.id,
    index: parent.children.findIndex((c) => c.id === node.id),
    of: parent.children.length,
  };
  const frame = flowing(node);
  if (frame) out.placed_by = `${frame}'s auto-layout`;
  else Object.assign(out, { x: num(node.x), y: num(node.y) });
  return out;
}

/** The layers of `node` and of its copy, walked side by side. */
function pairs(from: BaseNode, to: BaseNode, out: [string, string][]): void {
  out.push([from.id, to.id]);
  if ("children" in from && "children" in to) {
    from.children.forEach((child, i) => {
      const copy = to.children[i];
      if (copy) pairs(child, copy, out);
    });
  }
}

export async function duplicateNode(args: Args): Promise<unknown> {
  const node = await sceneNode(args.id);
  const parent = typeof args.parent === "string" ? await parentFor(args.parent) : (node.parent as Parent | null);
  if (!parent) throw new Error(`${node.id} has no parent`);
  const copy = node.clone();
  try {
    // In front of its new siblings.
    parent.appendChild(copy);
    copy.x = typeof args.x === "number" ? args.x : node.x;
    copy.y = typeof args.y === "number" ? args.y : node.y;
    if (typeof args.name === "string") copy.name = args.name;
  } catch (error) {
    copy.remove();
    throw error;
  }
  const all: [string, string][] = [];
  pairs(node, copy, all);
  const out: Record<string, unknown> = {
    id: copy.id,
    type: copy.type,
    name: copy.name,
    nodes: all.length,
    from: node.id,
    copies: all.slice(0, SHOWN).map(([from, id]) => ({ from, id })),
  };
  if (all.length > SHOWN) {
    out.omitted = { reason: "copies", remaining: all.length - SHOWN, how: "read_node the copy with depth to find the rest" };
  }
  return out;
}

export async function groupNodes(args: Args): Promise<unknown> {
  const ids = (args.ids as unknown[]).map(String);
  const nodes: SceneNode[] = [];
  for (const id of ids) nodes.push(await sceneNode(id, "ids"));
  const parent = nodes[0].parent;
  const apart = nodes.find((n) => n.parent?.id !== parent?.id);
  if (!parent || apart) throw new Error(`ids: ${apart?.id ?? nodes[0].id} is not in the same parent as ${nodes[0].id}; group layers that share a parent`);
  const group = figma.group(nodes, parent as Parent);
  group.name = typeof args.name === "string" ? args.name : "Group";
  return { id: group.id, type: group.type, name: group.name, children: ids };
}

export async function ungroup(args: Args): Promise<unknown> {
  const node = await sceneNode(args.id);
  if (node.type !== "GROUP") throw new Error(`${node.id} is a ${node.type}, not a group`);
  const parent = node.parent;
  const id = node.id;
  const children = figma.ungroup(node).map((c) => c.id);
  return { removed: id, children, parent: parent?.id };
}

/** Who still uses a component going with `going`: its instances outside it. */
async function users(going: SceneNode[]): Promise<string[]> {
  const found: string[] = [];
  for (const root of going) {
    const components: ComponentNode[] = [];
    if (root.type === "COMPONENT") components.push(root);
    if ("findAllWithCriteria" in root) components.push(...root.findAllWithCriteria({ types: ["COMPONENT"] }));
    for (const component of components) {
      for (const instance of await component.getInstancesAsync()) {
        if (!going.some((g) => isInside(instance, g))) found.push(`${instance.id} (an instance of ${component.id})`);
      }
    }
  }
  return found;
}

export async function deleteNode(args: Args): Promise<unknown> {
  const ids = [String(args.id), ...(Array.isArray(args.also) ? args.also.map(String) : [])];
  const going: BaseNode[] = [];
  for (const id of ids) going.push(await nodeFor(id));
  const layers = going.filter((n): n is SceneNode => n.type !== "PAGE" && n.type !== "DOCUMENT");
  if (going.some((n) => n.type === "DOCUMENT")) throw new Error("the document itself is not deleted");
  // As on a file: refused while something else still points at it.
  const using = await users(layers);
  if (using.length) throw new Error(`${ids[0]} is still used by ${using.slice(0, 10).join(", ")}${using.length > 10 ? ` and ${using.length - 10} more` : ""}; delete them too with \`also\`, or first`);
  const removed: string[] = [];
  for (const node of going) {
    if (node.removed) continue;
    removed.push(node.id);
    if ("findAll" in node) removed.push(...(node as ChildrenMixin).findAll(() => true).map((n) => n.id));
    node.remove();
  }
  return { removed };
}
