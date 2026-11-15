// list_open's entry for the open file, and render_node: what Figma itself
// knows and draws.
import { Args, nodeFor } from "./nodes";

/** The page the person is on and what they have selected, read now. */
export async function listOpen(): Promise<unknown> {
  const page = figma.currentPage;
  return {
    page: { id: page.id, name: page.name },
    selection: page.selection.map((node) => ({ id: node.id, name: node.name, type: node.type })),
  };
}

/** A node drawn by Figma as a PNG, scaled down to fit `max_size`. */
export async function renderNode(args: Args): Promise<unknown> {
  const node = await nodeFor(String(args.id));
  if (!("exportAsync" in node)) throw new Error(`${node.id} is a ${node.type}, which Figma does not draw`);
  const drawn = node as SceneNode;
  const maxSize = typeof args.max_size === "number" ? args.max_size : 2048;
  const wanted = typeof args.scale === "number" ? args.scale : 1;
  const largest = Math.max("width" in drawn ? drawn.width : 1, "height" in drawn ? drawn.height : 1, 1);
  const scale = Math.min(wanted, maxSize / largest);
  const bytes = await drawn.exportAsync({ format: "PNG", constraint: { type: "SCALE", value: scale } });
  return {
    image: { mime: "image/png", base64: figma.base64Encode(bytes) },
    caption: {
      id: node.id,
      name: node.name,
      width: Math.round(("width" in drawn ? drawn.width : 0) * scale),
      height: Math.round(("height" in drawn ? drawn.height : 0) * scale),
      scale,
      drawn_by: "Figma",
    },
  };
}
