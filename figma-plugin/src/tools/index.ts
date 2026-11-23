// Every tool the panel does, by grafta's name: each takes the tool's own
// arguments, already checked by grafta, and returns its own result keys.
import {
  addComponentProperty,
  componentInstances,
  createComponent,
  createInstance,
  createVariantSet,
  listComponents,
  overrideInstance,
  setVariant,
} from "./components";
import { createFrame, createPage, createSection, createShape, createText, createVector } from "./create";
import { getImage, summarize } from "./file";
import { listScreens, searchNodes } from "./find";
import { Args, Names, nodeFor, pathOf } from "./nodes";
import { checkContrast } from "./contrast";
import { listOpen, renderNode } from "./session";
import { setProperties } from "./set";
import { deleteNode, duplicateNode, groupNodes, moveNode, ungroup } from "./structure";
import { createStyle, listStyles, setStyle } from "./styles";
import { createVariable, createVariableCollection, listVariables, setVariable, setVariableCollection } from "./variables";
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
  check_contrast: checkContrast,
  list_components: listComponents,
  component_instances: componentInstances,
  create_component: createComponent,
  create_variant_set: createVariantSet,
  add_component_property: addComponentProperty,
  create_instance: createInstance,
  set_variant: setVariant,
  override_instance: overrideInstance,
  list_variables: listVariables,
  create_variable_collection: createVariableCollection,
  create_variable: createVariable,
  set_variable: setVariable,
  set_variable_collection: setVariableCollection,
  list_styles: listStyles,
  create_style: createStyle,
  set_style: setStyle,
  summarize,
  get_image: getImage,
  create_page: createPage,
  create_section: createSection,
  create_frame: createFrame,
  create_text: createText,
  create_shape: createShape,
  create_vector: createVector,
  move_node: moveNode,
  duplicate_node: duplicateNode,
  group_nodes: groupNodes,
  ungroup,
  delete_node: deleteNode,
};
