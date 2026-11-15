// Every tool the panel does, by grafta's name: each takes the tool's own
// arguments, already checked by grafta, and returns its own result keys.
import { listScreens, searchNodes } from "./find";
import { Args, Names, nodeFor, pathOf } from "./nodes";
import { listOpen, renderNode } from "./session";
import { setProperties } from "./set";
import { view } from "./view";
import { DEFAULT_BUDGET } from "./paging";

export type ToolFunction = (args: Args) => Promise<unknown>;

async function readNode(args: Args): Promise<unknown> {
  const node = await nodeFor(String(args.id));
  const out = await view(
    node,
    {
      depth: typeof args.depth === "number" ? args.depth : 1,
      budget: typeof args.budget === "number" ? args.budget : DEFAULT_BUDGET,
      geometry: args.geometry === true,
      childrenFrom: typeof args.children_from === "number" ? args.children_from : 0,
    },
    new Names(),
  );
  out.path = pathOf(node);
  return out;
}

export const REGISTRY: Record<string, ToolFunction | undefined> = {
  list_open: listOpen,
  read_node: readNode,
  search_nodes: searchNodes,
  list_screens: listScreens,
  set_properties: setProperties,
  render_node: renderNode,
};
