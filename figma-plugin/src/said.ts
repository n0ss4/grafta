// What a call did, in a designer's words: a verb and the layer it was done
// to, by the name Figma shows. Said from the tool, its arguments and its
// result alone; where none of them names the layer, it is "a layer", never a
// guess. The tool's own name is kept beside it for whoever is debugging.
import type { Said, SaidKind } from "./protocol";

type Args = Record<string, unknown>;

/** Each tool: what kind of thing it does, and how it is said. `{}` is where the name goes. */
const PHRASES: Record<string, { kind: SaidKind; done: string; failed: string; none?: string }> = {
  list_open: { kind: "look", done: "Looked at the open files", failed: "Could not look at the open files" },
  summarize: { kind: "look", done: "Looked through the file", failed: "Could not look through the file" },
  list_screens: { kind: "look", done: "Looked at the screens", failed: "Could not look at the screens" },
  search_nodes: { kind: "look", done: "Searched the file", failed: "Could not search the file" },
  read_node: { kind: "look", done: "Looked at {}", failed: "Could not look at {}", none: "a layer" },
  list_components: { kind: "look", done: "Looked at the components", failed: "Could not look at the components" },
  component_instances: { kind: "look", done: "Looked for uses of {}", failed: "Could not look for uses of {}", none: "a component" },
  list_variables: { kind: "look", done: "Looked at the variables", failed: "Could not look at the variables" },
  list_styles: { kind: "look", done: "Looked at the styles", failed: "Could not look at the styles" },
  get_image: { kind: "look", done: "Looked at an image", failed: "Could not look at an image" },
  render_node: { kind: "look", done: "Viewed {}", failed: "Could not view {}", none: "a layer" },
  check_contrast: { kind: "look", done: "Checked the contrast of {}", failed: "Could not check the contrast of {}", none: "a layer" },
  create_page: { kind: "make", done: "Added page {}", failed: "Could not add page {}", none: "" },
  create_frame: { kind: "make", done: "Added frame {}", failed: "Could not add frame {}", none: "" },
  create_section: { kind: "make", done: "Added section {}", failed: "Could not add section {}", none: "" },
  create_text: { kind: "make", done: "Added text {}", failed: "Could not add text {}", none: "" },
  create_vector: { kind: "make", done: "Drew {}", failed: "Could not draw {}", none: "a vector" },
  create_shape: { kind: "make", done: "Drew {}", failed: "Could not draw {}", none: "a shape" },
  create_instance: { kind: "make", done: "Added an instance of {}", failed: "Could not add an instance of {}", none: "a component" },
  create_component: { kind: "make", done: "Made {} a component", failed: "Could not make {} a component", none: "a layer" },
  create_variant_set: { kind: "make", done: "Combined variants into {}", failed: "Could not combine variants into {}", none: "a set" },
  duplicate_node: { kind: "make", done: "Duplicated {}", failed: "Could not duplicate {}", none: "a layer" },
  group_nodes: { kind: "make", done: "Grouped layers into {}", failed: "Could not group layers", none: "a group" },
  create_variable_collection: { kind: "make", done: "Added variable collection {}", failed: "Could not add variable collection {}", none: "" },
  create_variable: { kind: "make", done: "Added variable {}", failed: "Could not add variable {}", none: "" },
  create_style: { kind: "make", done: "Added style {}", failed: "Could not add style {}", none: "" },
  set_properties: { kind: "change", done: "Changed {}", failed: "Could not change {}", none: "a layer" },
  override_instance: { kind: "change", done: "Changed {}", failed: "Could not change {}", none: "an instance" },
  set_variant: { kind: "change", done: "Switched the variant of {}", failed: "Could not switch the variant of {}", none: "an instance" },
  add_component_property: { kind: "change", done: "Added a property to {}", failed: "Could not add a property to {}", none: "a component" },
  set_variable: { kind: "change", done: "Changed variable {}", failed: "Could not change variable {}", none: "" },
  set_variable_collection: { kind: "change", done: "Changed collection {}", failed: "Could not change collection {}", none: "" },
  set_style: { kind: "change", done: "Changed style {}", failed: "Could not change style {}", none: "" },
  ungroup: { kind: "change", done: "Ungrouped {}", failed: "Could not ungroup {}", none: "a group" },
  move_node: { kind: "move", done: "Moved {}", failed: "Could not move {}", none: "a layer" },
  delete_node: { kind: "remove", done: "Deleted {}", failed: "Could not delete {}", none: "a layer" },
};

/** set_properties' keys, by what a designer calls them; a key not here is not named. */
const WHAT: Record<string, string> = {
  fills: "fill", fillStyle: "fill", strokes: "border", strokeStyle: "border", strokeWeight: "border", strokeAlign: "border", dashPattern: "border",
  cornerRadius: "corners", topLeftRadius: "corners", topRightRadius: "corners", bottomLeftRadius: "corners", bottomRightRadius: "corners",
  x: "position", y: "position", width: "size", height: "size",
  layoutMode: "auto layout", itemSpacing: "auto layout", counterAxisSpacing: "auto layout", padding: "auto layout",
  paddingTop: "auto layout", paddingRight: "auto layout", paddingBottom: "auto layout", paddingLeft: "auto layout",
  primaryAxisAlignItems: "auto layout", counterAxisAlignItems: "auto layout", layoutWrap: "auto layout",
  effects: "effects", effectStyle: "effects", opacity: "opacity", visible: "visibility", name: "name",
  characters: "text", runs: "text", fontFamily: "font", fontStyle: "font", fontSize: "font", textStyle: "font",
  lineHeight: "spacing", letterSpacing: "spacing", textAlignHorizontal: "alignment", textAlignVertical: "alignment",
  reactions: "prototype", explicitVariableModes: "mode",
};

/** What kind of thing a tool does: a look, or one of the writes. */
export function kindOf(tool: string): SaidKind {
  return PHRASES[tool]?.kind ?? "change";
}

/** A phrase's first verb, past, as going on ("Changing") and as asked for ("change"). */
const VERBS: Record<string, [string, string]> = {
  Looked: ["Looking", "look"], Checked: ["Checking", "check"], Searched: ["Searching", "search"], Viewed: ["Viewing", "view"],
  Added: ["Adding", "add"], Drew: ["Drawing", "draw"], Made: ["Making", "make"], Combined: ["Combining", "combine"],
  Duplicated: ["Duplicating", "duplicate"], Grouped: ["Grouping", "group"], Changed: ["Changing", "change"],
  Switched: ["Switching", "switch"], Ungrouped: ["Ungrouping", "ungroup"], Moved: ["Moving", "move"], Deleted: ["Deleting", "delete"],
};

/** "Changed the fill of “Card”" as "Changing the fill of “Card”" (going) or "change the fill of “Card”" (asked). */
export function tense(text: string, form: "going" | "asked"): string {
  const [first, ...rest] = text.split(" ");
  const verb = VERBS[first];
  if (!verb) return form === "asked" ? text.charAt(0).toLowerCase() + text.slice(1) : text;
  return [form === "going" ? verb[0] : verb[1], ...rest].join(" ");
}

/** The name of the layer a call is about, read before the call: a layer it deletes is gone after. */
export async function before(args: Args): Promise<string | undefined> {
  if (typeof args.id !== "string") return undefined;
  try {
    return (await figma.getNodeByIdAsync(args.id))?.name;
  } catch {
    return undefined;
  }
}

function named(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function short(text: string): string {
  const line = text.split("\n")[0];
  return line.length > 40 ? `${line.slice(0, 39)}…` : line;
}

export function said(tool: string, args: Args, result: unknown, error: string | undefined, layer: string | undefined): Said {
  const phrase = PHRASES[tool];
  if (!phrase) return { kind: "change", text: tool, ok: error === undefined };
  const out = (result ?? {}) as Args;
  const name = layer ?? named(out.name) ?? named(args.name) ?? named(args.characters);
  const target = name ? `“${short(name)}”` : (phrase.none ?? "");
  let text = (error === undefined ? phrase.done : phrase.failed).replace("{}", target).replace(/\s+$/, "");
  if (tool === "set_properties" && error === undefined) {
    const props = Object.keys((args.properties ?? {}) as Args);
    const what = [...new Set(props.map((p) => WHAT[p]).filter((w): w is string => w !== undefined))];
    if (what.length && what.length <= 3) text = `Changed the ${what.length > 1 ? `${what.slice(0, -1).join(", ")} and ${what[what.length - 1]}` : what[0]} of ${target}`;
  }
  return { kind: phrase.kind, text, ok: error === undefined };
}
