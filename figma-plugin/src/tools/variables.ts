// The file's variables and their collections, on the open file. A plan's
// limit (a second mode) is Figma's refusal, passed on in its words (R5).
import { Args, bare, isId, num } from "./nodes";
import { hex, parseHex } from "./paint";
import { asked, bytes, pageAsync } from "./paging";

async function collections(): Promise<VariableCollection[]> {
  return figma.variables.getLocalVariableCollectionsAsync();
}

async function collectionNamed(wanted: string): Promise<VariableCollection> {
  const found = (await collections()).find((c) => c.name === wanted || isId(c.id, wanted));
  if (!found) throw new Error(`no variable collection named \`${wanted}\``);
  return found;
}

function collectionJson(collection: VariableCollection): Record<string, unknown> {
  return {
    id: bare(collection.id),
    name: collection.name,
    modes: collection.modes.map((m) => ({ id: m.modeId, name: m.name })),
    variables: collection.variableIds.length,
  };
}

/** A variable as list_variables shows it: values by mode name, colours in hex, an alias by name. */
async function variableJson(variable: Variable): Promise<Record<string, unknown>> {
  const collection = await figma.variables.getVariableCollectionByIdAsync(variable.variableCollectionId);
  const values: Record<string, unknown> = {};
  for (const [modeId, value] of Object.entries(variable.valuesByMode)) {
    const mode = collection?.modes.find((m) => m.modeId === modeId)?.name ?? modeId;
    if (typeof value === "object" && value !== null && "type" in value && value.type === "VARIABLE_ALIAS") {
      values[mode] = { alias: (await figma.variables.getVariableByIdAsync(value.id))?.name ?? value.id };
    } else if (typeof value === "object" && value !== null && "r" in value) {
      const a = "a" in value ? value.a : 1;
      values[mode] = a < 1 ? { color: hex(value), opacity: num(a) } : hex(value);
    } else {
      values[mode] = typeof value === "number" ? num(value) : value;
    }
  }
  return { id: bare(variable.id), name: variable.name, collection: collection?.name ?? variable.variableCollectionId, type: variable.resolvedType, values };
}

export async function listVariables(args: Args): Promise<unknown> {
  const wanted = typeof args.collection === "string" ? args.collection : undefined;
  const shown = (await collections()).filter((c) => wanted === undefined || c.name === wanted || isId(c.id, wanted));
  if (wanted !== undefined && !shown.length) throw new Error(`no variable collection named \`${wanted}\``);
  const ids = new Set(shown.map((c) => c.id));
  const variables = (await figma.variables.getLocalVariablesAsync()).filter((v) => ids.has(v.variableCollectionId));
  const collectionsJson = shown.map((c) => ({ id: bare(c.id), name: c.name, modes: c.modes.map((m) => m.name), variables: c.variableIds.length }));
  const out = await pageAsync(variables, asked(args, 200, "v"), "v", bytes(collectionsJson) + 32, variableJson);
  out.collections = collectionsJson;
  return out;
}

export async function createVariableCollection(args: Args): Promise<unknown> {
  const modes = args.modes as string[];
  const collection = figma.variables.createVariableCollection(String(args.name));
  try {
    collection.renameMode(collection.modes[0].modeId, modes[0]);
    for (const mode of modes.slice(1)) collection.addMode(mode);
  } catch (error) {
    collection.remove();
    throw error;
  }
  return { ...collectionJson(collection), next: "create_variable with collection: this id or name, and a value for each mode" };
}

/** The file's own variable by id, or by name when only one has it. */
async function variableNamed(wanted: string): Promise<Variable> {
  const byId = await figma.variables.getVariableByIdAsync(wanted.includes(":") && !wanted.startsWith("VariableID:") ? `VariableID:${wanted}` : wanted);
  if (byId && !byId.remote) return byId;
  const found = (await figma.variables.getLocalVariablesAsync()).filter((v) => v.name === wanted);
  if (found.length > 1) throw new Error(`${found.length} variables are named \`${wanted}\`; pass one id: ${found.map((v) => bare(v.id)).join(", ")}`);
  if (!found.length) throw new Error(`no local variable \`${wanted}\`: list_variables lists them`);
  return found[0];
}

/** Values by mode name, written into `variable`. */
async function writeValues(variable: Variable, collection: VariableCollection, values: Record<string, unknown>): Promise<void> {
  for (const [mode, value] of Object.entries(values)) {
    const found = collection.modes.find((m) => m.name === mode || m.modeId === mode);
    if (!found) throw new Error(`values.${mode}: ${collection.name} has no such mode; it has ${collection.modes.map((m) => m.name).join(", ")}`);
    variable.setValueForMode(found.modeId, await valueFor(`values.${mode}`, variable.resolvedType, value));
  }
}

async function valueFor(at: string, type: VariableResolvedDataType, value: unknown): Promise<VariableValue> {
  if (typeof value === "object" && value !== null && "alias" in value) {
    const target = await variableNamed(String((value as { alias: unknown }).alias));
    if (target.resolvedType !== type) throw new Error(`${at}: ${target.name} is a ${target.resolvedType}, not a ${type}`);
    return figma.variables.createVariableAlias(target);
  }
  switch (type) {
    case "COLOR":
      return parseHex(at, value);
    case "FLOAT":
      if (typeof value !== "number") throw new Error(`${at}: expected a number, got ${JSON.stringify(value)}`);
      return value;
    case "BOOLEAN":
      if (typeof value !== "boolean") throw new Error(`${at}: expected true or false, got ${JSON.stringify(value)}`);
      return value;
    default:
      if (typeof value !== "string") throw new Error(`${at}: expected a string, got ${JSON.stringify(value)}`);
      return value;
  }
}

export async function createVariable(args: Args): Promise<unknown> {
  const collection = await collectionNamed(String(args.collection));
  const variable = figma.variables.createVariable(String(args.name), collection, String(args.type) as VariableResolvedDataType);
  try {
    await writeValues(variable, collection, (args.values ?? {}) as Record<string, unknown>);
  } catch (error) {
    variable.remove();
    throw error;
  }
  return variableJson(variable);
}

export async function setVariable(args: Args): Promise<unknown> {
  const variable = await variableNamed(String(args.id));
  const collection = await figma.variables.getVariableCollectionByIdAsync(variable.variableCollectionId);
  if (!collection) throw new Error(`${variable.name}'s collection is not in this file`);
  const before = { name: variable.name, values: { ...variable.valuesByMode } };
  try {
    if (typeof args.name === "string") variable.name = args.name;
    if (args.values !== undefined) await writeValues(variable, collection, args.values as Record<string, unknown>);
  } catch (error) {
    variable.name = before.name;
    for (const [mode, value] of Object.entries(before.values)) variable.setValueForMode(mode, value);
    throw error;
  }
  return variableJson(variable);
}

export async function setVariableCollection(args: Args): Promise<unknown> {
  const collection = await collectionNamed(String(args.id));
  const name = collection.name;
  if (typeof args.name === "string") collection.name = args.name;
  if (typeof args.add_mode === "string") {
    try {
      collection.addMode(args.add_mode);
    } catch (error) {
      collection.name = name;
      throw error;
    }
  }
  return collectionJson(collection);
}
