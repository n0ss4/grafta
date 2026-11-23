// search_nodes and list_screens on the open file: a page of Figma's at a
// time, so no call walks a whole kit in one pass (research R4).
import { Args, nodeFor, pathOf } from "./nodes";
import { asked, bytes, DEFAULT_BUDGET, page } from "./paging";

/** The node types a search may name, in the plugin API's spelling. */
const TYPES = new Set([
  "DOCUMENT", "PAGE", "FRAME", "GROUP", "SECTION", "COMPONENT", "COMPONENT_SET", "INSTANCE", "TEXT", "RECTANGLE",
  "ELLIPSE", "LINE", "POLYGON", "STAR", "VECTOR", "BOOLEAN_OPERATION", "SLICE", "STICKY", "SHAPE_WITH_TEXT",
  "CONNECTOR", "TABLE", "WIDGET", "EMBED", "LINK_UNFURL", "MEDIA", "STAMP", "HIGHLIGHT", "WASHI_TAPE", "CODE_BLOCK",
]);

/** How long a search across pages runs before it gives a cursor for the rest. */
const SEARCH_MS = 2000;

type Found = { id: string; name: string; type: string; path: string };

function matcher(args: Args): (node: BaseNode) => boolean {
  const name = typeof args.name === "string" ? args.name.toLowerCase() : undefined;
  const given = typeof args.type === "string" ? args.type : undefined;
  const type = given?.toUpperCase() === "CANVAS" ? "PAGE" : given?.toUpperCase();
  if (given !== undefined && !TYPES.has(type ?? "")) {
    throw new Error(
      `parameter "type": \`${given}\` is not a node type. Use FRAME, TEXT, INSTANCE, COMPONENT, COMPONENT_SET, PAGE, SECTION, GROUP, RECTANGLE, ELLIPSE, VECTOR, …`,
    );
  }
  const text = lowered(args.text);
  return (node) =>
    (name === undefined || node.name.toLowerCase().includes(name)) &&
    (type === undefined || node.type === type) &&
    (text === undefined || (node.type === "TEXT" && node.characters.toLowerCase().includes(text)));
}

const lowered = (given: unknown): string | undefined => (typeof given === "string" ? given.toLowerCase() : undefined);

type Alias = { id: string };
const paintAliases = (paint: Paint): Alias[] => ("boundVariables" in paint ? aliases(Object.values(paint.boundVariables ?? {})) : []);
const aliases = (value: unknown): Alias[] =>
  Array.isArray(value) ? value.flatMap(aliases) : value && typeof value === "object" && "id" in value ? [value as Alias] : [];

/** The ids of the styles a layer uses, its own and its text's runs'. */
function styleIds(node: SceneNode): string[] {
  const ids: unknown[] = [];
  for (const key of ["fillStyleId", "strokeStyleId", "effectStyleId", "textStyleId"] as const) {
    if (key in node) ids.push((node as unknown as Record<string, unknown>)[key]);
  }
  if (node.type === "TEXT") {
    for (const run of node.getStyledTextSegments(["textStyleId", "fillStyleId"])) ids.push(run.textStyleId, run.fillStyleId);
  }
  return ids.filter((id): id is string => typeof id === "string" && id !== "");
}

/** The ids of the variables a layer binds: in any field, in its paints, in its text's runs. */
function variableIds(node: SceneNode): string[] {
  const found: Alias[] = [];
  if ("boundVariables" in node) found.push(...Object.values(node.boundVariables ?? {}).flatMap(aliases));
  for (const key of ["fills", "strokes"] as const) {
    const paints = (node as unknown as Record<string, unknown>)[key];
    if (Array.isArray(paints)) for (const paint of paints as Paint[]) found.push(...paintAliases(paint));
  }
  if (node.type === "TEXT") {
    for (const run of node.getStyledTextSegments(["fills", "boundVariables"])) {
      found.push(...aliases(Object.values(run.boundVariables ?? {})));
      for (const paint of run.fills) found.push(...paintAliases(paint));
    }
  }
  return found.map((alias) => alias.id);
}

/**
 * What a search asks of the styles, variables and component a layer uses:
 * names Figma gives only asynchronously, each looked up once.
 */
function usesMatcher(args: Args): ((node: BaseNode) => Promise<boolean>) | undefined {
  const style = lowered(args.style);
  const variable = lowered(args.variable);
  const component = typeof args.component === "string" ? args.component : undefined;
  if (style === undefined && variable === undefined && component === undefined) return undefined;
  const names = new Map<string, Promise<string | undefined>>();
  const once = (key: string, look: () => Promise<string | undefined>) => {
    if (!names.has(key)) names.set(key, look().catch(() => undefined));
    return names.get(key)!;
  };
  const any = async (ids: string[], look: (id: string) => Promise<string | undefined>, wanted: string) => {
    for (const id of ids) if ((await look(id))?.toLowerCase().includes(wanted)) return true;
    return false;
  };
  const styleName = (id: string) => once(`s${id}`, async () => (await figma.getStyleByIdAsync(id))?.name);
  const variableName = (id: string) => once(`v${id}`, async () => (await figma.variables.getVariableByIdAsync(id))?.name);
  return async (base) => {
    if (base.type === "DOCUMENT" || base.type === "PAGE") return false;
    const node = base as SceneNode;
    if (style !== undefined && !(await any(styleIds(node), styleName, style))) return false;
    if (variable !== undefined && !(await any(variableIds(node), variableName, variable))) return false;
    if (component !== undefined) {
      if (node.type !== "INSTANCE") return false;
      const main = await node.getMainComponentAsync();
      if (!main) return false;
      const set = main.parent?.type === "COMPONENT_SET" ? main.parent : undefined;
      const wanted = component.toLowerCase();
      const named = [main, set].some((c) => c?.name.toLowerCase().includes(wanted));
      if (main.id !== component && set?.id !== component && !named) return false;
    }
    return true;
  };
}

async function narrowed(found: BaseNode[], uses: ((node: BaseNode) => Promise<boolean>) | undefined): Promise<BaseNode[]> {
  if (!uses) return found;
  const kept: BaseNode[] = [];
  for (const node of found) if (await uses(node)) kept.push(node);
  return kept;
}

const shown = (node: BaseNode): Found => ({ id: node.id, name: node.name, type: node.type, path: pathOf(node) });

export async function searchNodes(args: Args): Promise<unknown> {
  const matches = matcher(args);
  const uses = usesMatcher(args);
  if (typeof args.within === "string") {
    const root = await nodeFor(args.within);
    const found = "findAll" in root ? (root as ChildrenMixin).findAll(matches) : [];
    return page(await narrowed(found, uses), asked(args, 50, "w"), "w", 0, shown);
  }
  return acrossPages(args, matches, uses);
}

/**
 * The file's pages in order from the cursor's, each searched whole, until the
 * page of results is full or the time is spent; the cursor names the page and
 * the place in it to go on from.
 */
async function acrossPages(
  args: Args,
  matches: (node: BaseNode) => boolean,
  uses: ((node: BaseNode) => Promise<boolean>) | undefined,
): Promise<unknown> {
  const limit = typeof args.limit === "number" ? args.limit : 50;
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
    if (at > pageAt && (Date.now() - started > SEARCH_MS || results.length >= limit)) {
      next = `p${at}.0`;
      break;
    }
    const pageNode = pages[at];
    await pageNode.loadAsync();
    const found = await narrowed([...(matches(pageNode) ? [pageNode] : []), ...pageNode.findAll(matches)], uses);
    total += found.length;
    searched += 1;
    const cut = page(found, { limit: limit - results.length, offset: at === pageAt ? offset : 0, budget }, "", 0, shown);
    results.push(...(cut.results as unknown[]));
    if (typeof cut.next_cursor === "string") {
      next = `p${at}.${cut.next_cursor}`;
      break;
    }
  }
  const out: Record<string, unknown> = { total, results, pages_searched: searched, pages: pages.length };
  if (next) {
    out.next_cursor = next;
    out.omitted = {
      reason: "page",
      pages_remaining: pages.length - Number(next.slice(1).split(".")[0]),
      how: "call again with this next_cursor: the open file is searched a page at a time",
    };
  }
  return out;
}

export async function listScreens(args: Args): Promise<unknown> {
  const pageNode = await nodeFor(String(args.page));
  if (pageNode.type !== "PAGE") throw new Error(`${pageNode.id} is a ${pageNode.type}, not a page: summarize lists the pages`);
  const pageJson = { id: pageNode.id, name: pageNode.name };
  const reserve = bytes(pageJson);
  const out = page([...pageNode.children], asked(args, 100, "s"), "s", reserve, (node) => ({
    id: node.id,
    name: node.name,
    type: node.type,
    width: "width" in node ? node.width : 0,
    height: "height" in node ? node.height : 0,
    visible: node.visible,
  }));
  out.page = pageJson;
  return out;
}
