import type { Family, Tool } from "./types";

/** A tool the search kept, and whether it matched only outside its name. */
export interface Found {
  tool: Tool;
  /** True when the query is in its arguments or description, not its name. */
  inside: boolean;
}

export interface FoundFamily extends Omit<Family, "tools"> {
  tools: Found[];
}

/**
 * Families narrowed to the tools whose name, an argument's name, or whose own or
 * an argument's description holds the query, ignoring case.
 */
export function filterFamilies(families: Family[], query: string): FoundFamily[] {
  const q = query.trim().toLowerCase();
  const has = (text: string) => text.toLowerCase().includes(q);
  return families
    .map((family) => ({
      ...family,
      tools: family.tools.flatMap((tool): Found[] => {
        if (!q) return [{ tool, inside: false }];
        if (has(tool.name)) return [{ tool, inside: false }];
        const inside =
          has(tool.description) || tool.arguments.some((arg) => has(arg.name) || has(arg.description));
        return inside ? [{ tool, inside: true }] : [];
      }),
    }))
    .filter((family) => family.tools.length > 0);
}
