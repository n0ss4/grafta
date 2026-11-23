// summarize and get_image on the open file.
import { Args } from "./nodes";

/** How long summarize counts before it names the pages left. */
const COUNT_MS = 2000;

/** The largest image given as one, as on a file. */
const MAX_IMAGE_BYTES = 4 * 1024 * 1024;

const COUNTED = { COMPONENT: "components", COMPONENT_SET: "component_sets", INSTANCE: "instances", TEXT: "text_nodes" } as const;

/** An overview of the open file, a page at a time; the pages not counted in time are named. */
export async function summarize(): Promise<unknown> {
  const started = Date.now();
  const pages: Record<string, unknown>[] = [];
  const left: { id: string; name: string }[] = [];
  const totals = { nodes: 1, components: 0, component_sets: 0, instances: 0, text_nodes: 0 };
  for (const page of figma.root.children) {
    if (pages.length && Date.now() - started > COUNT_MS) {
      left.push({ id: page.id, name: page.name });
      continue;
    }
    await page.loadAsync();
    const all = page.findAll(() => true);
    for (const node of all) {
      const key = COUNTED[node.type as keyof typeof COUNTED];
      if (key) totals[key] += 1;
    }
    totals.nodes += all.length + 1;
    pages.push({ id: page.id, name: page.name, nodes: all.length + 1, top_level: page.children.length });
  }
  const [collections, variables, styles] = await Promise.all([
    figma.variables.getLocalVariableCollectionsAsync(),
    figma.variables.getLocalVariablesAsync(),
    Promise.all([figma.getLocalPaintStylesAsync(), figma.getLocalTextStylesAsync(), figma.getLocalEffectStylesAsync(), figma.getLocalGridStylesAsync()]),
  ]);
  const out: Record<string, unknown> = {
    file: figma.root.name,
    product: "design",
    pages,
    ...totals,
    variable_collections: collections.length,
    variables: variables.length,
    styles: styles.reduce((n, s) => n + s.length, 0),
  };
  const omitted: Record<string, unknown>[] = [{ reason: "not counted", keys: ["images", "prototype"], how: "the open file does not list its images or its prototype in one call; read_node shows a node's fills and reactions" }];
  if (left.length) {
    omitted.push({ reason: "time", pages: left, how: `counted a page at a time for ${COUNT_MS / 1000} s: the totals are for the pages listed; list_screens or search_nodes reads the rest` });
  }
  out.omitted = omitted;
  return out;
}

function mime(bytes: Uint8Array): string {
  const starts = (...b: number[]) => b.every((v, i) => bytes[i] === v);
  if (starts(0x89, 0x50, 0x4e, 0x47)) return "image/png";
  if (starts(0xff, 0xd8, 0xff)) return "image/jpeg";
  if (starts(0x47, 0x49, 0x46, 0x38)) return "image/gif";
  if (bytes.length > 12 && starts(0x52, 0x49, 0x46, 0x46) && String.fromCharCode(...bytes.slice(8, 12)) === "WEBP") return "image/webp";
  return "application/octet-stream";
}

/** An image the file holds, by the hash a fill carries. */
export async function getImage(args: Args): Promise<unknown> {
  const hash = String(args.hash).replace(/^images\//, "");
  const image = figma.getImageByHash(hash);
  if (!image) throw new Error(`no image ${hash} in ${figma.root.name}: read_node shows a fill's imageHash`);
  const bytes = await image.getBytesAsync();
  const type = mime(bytes);
  if (bytes.length > MAX_IMAGE_BYTES) return { too_large: { bytes: bytes.length, limit: MAX_IMAGE_BYTES }, mime: type };
  return { image: { mime: type, base64: figma.base64Encode(bytes) } };
}
