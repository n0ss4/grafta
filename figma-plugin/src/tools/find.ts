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
  return (node) => (name === undefined || node.name.toLowerCase().includes(name)) && (type === undefined || node.type === type);
}

const shown = (node: BaseNode): Found => ({ id: node.id, name: node.name, type: node.type, path: pathOf(node) });

export async function searchNodes(args: Args): Promise<unknown> {
  const matches = matcher(args);
  if (typeof args.within === "string") {
    const root = await nodeFor(args.within);
    const found = "findAll" in root ? (root as ChildrenMixin).findAll(matches) : [];
    return page(found, asked(args, 50, "w"), "w", 0, shown);
  }
  return acrossPages(args, matches);
}

/**
 * The file's pages in order from the cursor's, each searched whole, until the
 * page of results is full or the time is spent; the cursor names the page and
 * the place in it to go on from.
 */
async function acrossPages(args: Args, matches: (node: BaseNode) => boolean): Promise<unknown> {
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
    const found = [...(matches(pageNode) ? [pageNode] : []), ...pageNode.findAll(matches)];
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
