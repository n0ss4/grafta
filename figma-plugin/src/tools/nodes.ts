// Finding nodes, and the names a result shows for the ids Figma gives.

/** A tool's arguments, already checked by grafta against its schema. */
export type Args = Record<string, unknown>;

/** A node by id, or words saying it is not in the file. */
export async function nodeFor(id: string): Promise<BaseNode> {
  const node = await figma.getNodeByIdAsync(id);
  if (!node) throw new Error(`unknown node ${id} in ${figma.root.name}. Node ids look like 12:345; search_nodes finds them`);
  if (node.type === "PAGE") await node.loadAsync();
  return node;
}

/** The node's names from its page down, as grafta's `path`. */
export function pathOf(node: BaseNode): string {
  const names: string[] = [];
  for (let at: BaseNode | null = node; at && at.type !== "DOCUMENT"; at = at.parent) names.push(at.name);
  return names.reverse().join(" / ");
}

/**
 * A variable's or collection's id as grafta gives it on a file: Figma's
 * without its `VariableID:` or `VariableCollectionId:` prefix, so the same id
 * names it in both places.
 */
export function bare(id: string): string {
  return id.replace(/^(VariableID|VariableCollectionId):/, "");
}

/** Whether `given` names the variable or collection whose Figma id is `id`, in either spelling. */
export function isId(id: string, given: string): boolean {
  return id === given || bare(id) === given;
}

/** A node's own properties, read by name: what a type lacks reads as undefined. */
export function read(node: BaseNode, key: string): unknown {
  return key in node ? (node as unknown as Record<string, unknown>)[key] : undefined;
}

/** Names of styles and variables by id, read once per call. */
export class Names {
  private styles = new Map<string, string | null>();
  private variables = new Map<string, Variable | null>();
  private collections = new Map<string, VariableCollection | null>();

  async style(id: unknown): Promise<string | undefined> {
    if (typeof id !== "string" || id === "") return undefined;
    if (!this.styles.has(id)) this.styles.set(id, (await figma.getStyleByIdAsync(id))?.name ?? null);
    return this.styles.get(id) ?? id;
  }

  async variable(id: string): Promise<Variable | null> {
    if (!this.variables.has(id)) this.variables.set(id, await figma.variables.getVariableByIdAsync(id));
    return this.variables.get(id) ?? null;
  }

  /** A variable as grafta names it: its name, or its id when it is not found. */
  async label(alias: unknown): Promise<string | undefined> {
    const id = (alias as VariableAlias | undefined)?.id;
    if (typeof id !== "string") return undefined;
    return (await this.variable(id))?.name ?? id;
  }

  async collection(id: string): Promise<VariableCollection | null> {
    if (!this.collections.has(id)) this.collections.set(id, await figma.variables.getVariableCollectionByIdAsync(id));
    return this.collections.get(id) ?? null;
  }
}

/**
 * A number as grafta writes one: Figma keeps single precision, so the
 * shortest decimal that is the same single-precision number, never
 * `12.000000476837158`.
 */
export function num(value: number): number {
  if (!Number.isFinite(value) || Number.isInteger(value)) return value;
  const single = Math.fround(value);
  for (let digits = 1; digits <= 9; digits++) {
    const candidate = Number(single.toPrecision(digits));
    if (Math.fround(candidate) === single) return candidate;
  }
  return value;
}
