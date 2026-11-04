import type { Family } from "./types";

/** Families narrowed to the tools whose name, or an argument's name, holds the query. */
export function filterFamilies(families: Family[], query: string): Family[] {
  const q = query.trim().toLowerCase();
  if (!q) return families;
  return families
    .map((family) => ({
      ...family,
      tools: family.tools.filter(
        (tool) => tool.name.includes(q) || tool.arguments.some((arg) => arg.name.toLowerCase().includes(q)),
      ),
    }))
    .filter((family) => family.tools.length > 0);
}
