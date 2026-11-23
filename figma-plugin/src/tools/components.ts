// Components, variant sets, their properties and instances, on the open file.
import { Args, nodeFor, pathOf } from "./nodes";
import { paintsIn } from "./paint";
import { asked, DEFAULT_BUDGET, pageAsync } from "./paging";
import { loadFonts, Variables } from "./set";

/** How long a list across pages runs before it gives a cursor for the rest. */
const LIST_MS = 2000;

/** The variants of a set, or the component itself. */
function definitions(node: ComponentNode | ComponentSetNode): ComponentNode[] {
  return node.type === "COMPONENT_SET" ? node.children.filter((c): c is ComponentNode => c.type === "COMPONENT") : [node];
}

async function instancesOf(node: ComponentNode | ComponentSetNode): Promise<InstanceNode[]> {
  const all: InstanceNode[] = [];
  for (const component of definitions(node)) all.push(...(await component.getInstancesAsync()));
  return all;
}

function pageOf(node: BaseNode): PageNode | undefined {
  for (let at: BaseNode | null = node; at; at = at.parent) if (at.type === "PAGE") return at;
  return undefined;
}

async function componentJson(node: ComponentNode | ComponentSetNode): Promise<unknown> {
  const out: Record<string, unknown> = {
    id: node.id,
    name: node.name,
    type: node.type,
    width: node.width,
    height: node.height,
    instances: (await instancesOf(node)).length,
  };
  const page = pageOf(node);
  if (page) out.page = page.name;
  if (node.type === "COMPONENT_SET") {
    out.variants = definitions(node).length;
    const options = Object.fromEntries(Object.entries(node.variantGroupProperties).map(([k, g]) => [k, g.values]));
    if (Object.keys(options).length) out.variantOptions = options;
  }
  const properties = Object.entries(node.componentPropertyDefinitions)
    .filter(([, d]) => d.type !== "VARIANT")
    .map(([key, d]) => ({ name: key.split("#")[0], type: d.type, default: d.defaultValue }));
  if (properties.length) out.properties = properties;
  if (node.description) out.description = node.description;
  return out;
}

/** The file's components, a page of Figma's at a time, with a cursor for the rest. */
export async function listComponents(args: Args): Promise<unknown> {
  const name = typeof args.name === "string" ? args.name.toLowerCase() : undefined;
  const limit = typeof args.limit === "number" ? args.limit : 100;
  const budget = typeof args.budget === "number" ? args.budget : DEFAULT_BUDGET;
  let [pageAt, offset] = [0, 0];
  if (typeof args.cursor === "string") {
    const cursor = /^p(\d+)\.(\d+)$/.exec(args.cursor);
    if (!cursor) throw new Error(`parameter "cursor": \`${args.cursor}\` is not a cursor this toolkit issued`);
    [pageAt, offset] = [Number(cursor[1]), Number(cursor[2])];
  }
  const pages = figma.root.children;
  const started = Date.now();
  const results: unknown[] = [];
  let total = 0;
  let searched = 0;
  let next: string | undefined;
  for (let at = pageAt; at < pages.length; at++) {
    if (at > pageAt && (Date.now() - started > LIST_MS || results.length >= limit)) {
      next = `p${at}.0`;
      break;
    }
    await pages[at].loadAsync();
    const found = pages[at]
      .findAllWithCriteria({ types: ["COMPONENT", "COMPONENT_SET"] })
      .filter((n) => n.type === "COMPONENT_SET" || n.parent?.type !== "COMPONENT_SET")
      .filter((n) => name === undefined || n.name.toLowerCase().includes(name));
    total += found.length;
    searched += 1;
    const cut = await pageAsync(found, { limit: limit - results.length, offset: at === pageAt ? offset : 0, budget }, "", 0, componentJson);
    results.push(...(cut.results as unknown[]));
    if (typeof cut.next_cursor === "string") {
      next = `p${at}.${cut.next_cursor}`;
      break;
    }
  }
  const out: Record<string, unknown> = { total, results, pages_searched: searched, pages: pages.length };
  if (next) {
    out.next_cursor = next;
    out.omitted = { reason: "page", how: "call again with this next_cursor: the open file is listed a page at a time" };
  }
  return out;
}

async function componentFor(id: string): Promise<ComponentNode | ComponentSetNode> {
  const node = await nodeFor(id);
  if (node.type !== "COMPONENT" && node.type !== "COMPONENT_SET") throw new Error(`${node.id} is a ${node.type}, not a component or component set`);
  return node;
}

export async function componentInstances(args: Args): Promise<unknown> {
  const instances = await instancesOf(await componentFor(String(args.id)));
  return pageAsync(instances, asked(args, 50, "i"), "i", 0, async (node) => ({ id: node.id, name: node.name, path: pathOf(node) }));
}

export async function createComponent(args: Args): Promise<unknown> {
  const node = await nodeFor(String(args.id));
  if (node.type !== "FRAME") throw new Error(`${node.id} is a ${node.type}: only a frame is made a component`);
  const component = figma.createComponentFromNode(node);
  if (typeof args.name === "string") component.name = args.name;
  return {
    id: component.id,
    name: component.name,
    type: component.type,
    children: component.children.length,
    next: "create_instance with this id as the component; override_instance to change a copy",
  };
}

export async function createVariantSet(args: Args): Promise<unknown> {
  const variants = args.variants as { id: string; values: Record<string, string> }[];
  const components: ComponentNode[] = [];
  for (const variant of variants) {
    const node = await nodeFor(variant.id);
    if (node.type !== "COMPONENT") throw new Error(`variants: ${node.id} is a ${node.type}, not a component`);
    components.push(node);
  }
  const names = components.map((c) => c.name);
  components.forEach((component, i) => {
    component.name = Object.entries(variants[i].values)
      .map(([key, value]) => `${key}=${value}`)
      .join(", ");
  });
  const parent = components[0].parent;
  if (!parent || !("appendChild" in parent)) throw new Error(`${components[0].id} has no parent to hold the set`);
  let set: ComponentSetNode;
  try {
    set = figma.combineAsVariants(components, parent as BaseNode & ChildrenMixin);
  } catch (error) {
    components.forEach((component, i) => (component.name = names[i]));
    throw error;
  }
  set.name = String(args.name);
  return {
    id: set.id,
    name: set.name,
    type: set.type,
    variants: definitions(set).map((c) => ({ id: c.id, name: c.name })),
    next: "create_instance with a variant's id; set_variant to switch an instance to another",
  };
}

export async function addComponentProperty(args: Args): Promise<unknown> {
  const component = await componentFor(String(args.component));
  if (component.type === "COMPONENT" && component.parent?.type === "COMPONENT_SET") {
    throw new Error(`${component.id} is a variant: add the property to its set, ${component.parent.id}`);
  }
  const type = String(args.type) as "TEXT" | "BOOLEAN" | "INSTANCE_SWAP";
  let value = args.default as string | boolean;
  if (type === "INSTANCE_SWAP") {
    const swap = await swapTarget(String(value));
    value = swap.id;
  }
  const key = component.addComponentProperty(String(args.name), type, value);
  return {
    id: key,
    component: component.id,
    name: String(args.name),
    type,
    next: "bind a layer inside the component: set_properties componentPropertyReferences { characters | visible | mainComponent: <this name> }",
  };
}

/** A component for a swap, by id, or by name on the current page. */
export async function swapTarget(wanted: string): Promise<ComponentNode> {
  const byId = await figma.getNodeByIdAsync(wanted);
  if (byId?.type === "COMPONENT") return byId;
  const named = figma.currentPage.findAllWithCriteria({ types: ["COMPONENT"] }).filter((c) => c.name === wanted);
  if (named.length === 1) return named[0];
  throw new Error(
    named.length
      ? `${named.length} components on this page are named \`${wanted}\`; pass one id: ${named.map((c) => c.id).join(", ")}`
      : `no component \`${wanted}\` by that id, nor by that name on the current page: pass its id`,
  );
}

export async function createInstance(args: Args): Promise<unknown> {
  const component = await nodeFor(String(args.component));
  if (component.type === "COMPONENT_SET") throw new Error(`${component.id} is a component set: create an instance of one of its variants`);
  if (component.type !== "COMPONENT") throw new Error(`${component.id} is a ${component.type}, not a component`);
  const parent = await nodeFor(String(args.parent));
  if (!("appendChild" in parent)) throw new Error(`${parent.id} is a ${parent.type}, which holds no children`);
  const instance = component.createInstance();
  try {
    (parent as BaseNode & ChildrenMixin).appendChild(instance);
    if (typeof args.name === "string") instance.name = args.name;
    if (typeof args.x === "number") instance.x = args.x;
    if (typeof args.y === "number") instance.y = args.y;
  } catch (error) {
    instance.remove();
    throw error;
  }
  return { id: instance.id, type: instance.type, name: instance.name, width: instance.width, height: instance.height };
}

async function instanceFor(id: string): Promise<InstanceNode> {
  const node = await nodeFor(id);
  if (node.type !== "INSTANCE") throw new Error(`${node.id} is a ${node.type}, not an instance`);
  return node;
}

export async function setVariant(args: Args): Promise<unknown> {
  const instance = await instanceFor(String(args.id));
  instance.setProperties(args.values as Record<string, string>);
  const main = await instance.getMainComponentAsync();
  return { id: instance.id, variant: main?.id, variant_name: main?.name };
}

/** The layer an override names: an id in the component, or a path of ids through nested instances. */
async function layerFor(instance: InstanceNode, layer: unknown): Promise<SceneNode> {
  const path = Array.isArray(layer) ? layer.map(String) : [String(layer)];
  const id = path[0].startsWith("I") ? path[0] : `I${instance.id};${path.join(";")}`;
  const node = await figma.getNodeByIdAsync(id);
  if (!node) throw new Error(`layer: no layer ${path.join(" → ")} in the instance ${instance.id}; read_node on the instance lists its layers`);
  return node as SceneNode;
}

export async function overrideInstance(args: Args): Promise<unknown> {
  const instance = await instanceFor(String(args.id));
  const properties = args.properties as Record<string, unknown>;
  const keys = Object.keys(properties);
  if (!keys.length) throw new Error("nothing to change: pass properties");
  let targets: SceneNode[];
  if (args.layer !== undefined) {
    targets = [await layerFor(instance, args.layer)];
  } else {
    const wanted = keys.filter((k) => k !== "fills" && k !== "strokes");
    if (wanted.length) throw new Error(`properties.${wanted[0]}: name the layer it is for, with layer`);
    // Recolouring: every layer of the instance that has what is being set.
    targets = instance.findAll((n) => keys.some((k) => Array.isArray((n as unknown as Record<string, unknown>)[k]) && ((n as unknown as Record<string, unknown[]>)[k]).length > 0));
    if (!targets.length) throw new Error(`no layer of ${instance.id} has ${keys.join(" or ")} to recolour`);
  }
  const variables = new Variables();
  const undo: (() => void)[] = [];
  try {
    for (const target of targets) {
      const node = target as unknown as Record<string, unknown>;
      for (const key of keys) {
        const value = properties[key];
        if ((key === "fills" || key === "strokes") && args.layer === undefined && !((node[key] as unknown[] | undefined)?.length)) continue;
        if (key === "characters" && target.type !== "TEXT") throw new Error(`properties.characters: ${target.id} is a ${target.type}, not a text`);
        if (key === "characters") await loadFonts(target as TextNode);
        const given = key === "fills" || key === "strokes" ? await paintsIn(key, value, (name) => variables.named(name)) : value;
        const before = node[key];
        try {
          node[key] = given;
        } catch (error) {
          throw new Error(`properties.${key} on ${target.id}: ${error instanceof Error ? error.message : String(error)}`);
        }
        undo.push(() => (node[key] = before));
      }
    }
  } catch (error) {
    for (const step of undo.reverse()) step();
    throw new Error(`${error instanceof Error ? error.message : String(error)}. Nothing was changed`);
  }
  return { id: instance.id, layers: targets.map((t) => t.id), set: keys };
}
