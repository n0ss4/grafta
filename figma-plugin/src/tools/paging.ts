// A page of a longer list, as grafta's bound::page cuts one: by count and by
// bytes, saying how many there are and how to get the rest.

/** grafta's default result size, in bytes. */
export const DEFAULT_BUDGET = 32 * 1024;

/** The bytes of a value's JSON in UTF-8, as grafta measures a result: Figma's sandbox has no TextEncoder. */
export function bytes(value: unknown): number {
  const text = JSON.stringify(value) ?? "";
  let count = 0;
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    if (code < 0x80) count += 1;
    else if (code < 0x800) count += 2;
    else if (code >= 0xd800 && code < 0xdc00) {
      count += 4;
      i += 1;
    } else count += 3;
  }
  return count;
}

export interface Asked {
  limit: number;
  offset: number;
  budget: number;
}

/** What a list tool was asked: its `limit`, `cursor` and `budget`. */
export function asked(args: Record<string, unknown>, defaultLimit: number, prefix: string): Asked {
  const limit = typeof args.limit === "number" ? args.limit : defaultLimit;
  const budget = typeof args.budget === "number" ? args.budget : DEFAULT_BUDGET;
  let offset = 0;
  if (typeof args.cursor === "string") {
    const match = new RegExp(`^${prefix}(\\d+)$`).exec(args.cursor);
    if (!match) throw new Error(`parameter "cursor": \`${args.cursor}\` is not a cursor this toolkit issued`);
    offset = Number(match[1]);
  }
  return { limit, offset, budget };
}

/** One page of `items`, `reserve` bytes kept for what the caller adds. */
export function page<T>(items: T[], how: Asked, prefix: string, reserve: number, item: (value: T) => unknown): Record<string, unknown> {
  const results: unknown[] = [];
  let used = 256 + reserve;
  let next = how.offset;
  for (const value of items.slice(how.offset, how.offset + how.limit)) {
    const shown = item(value);
    const size = bytes(shown) + 1;
    if (used + size > how.budget && results.length) break;
    used += size;
    results.push(shown);
    next += 1;
  }
  const out: Record<string, unknown> = { total: items.length, results };
  if (next < items.length) {
    out.next_cursor = `${prefix}${next}`;
    out.omitted = { reason: "page", remaining: items.length - next, how: "call again with this next_cursor" };
  }
  return out;
}

/** [`page`], when an item is shown by reading Figma: only the page's items are read. */
export async function pageAsync<T>(
  items: T[],
  how: Asked,
  prefix: string,
  reserve: number,
  item: (value: T) => Promise<unknown>,
): Promise<Record<string, unknown>> {
  const shown: unknown[] = [];
  for (const value of items.slice(how.offset, how.offset + how.limit)) shown.push(await item(value));
  const out = page(shown, { ...how, offset: 0 }, prefix, reserve, (value) => value);
  const next = how.offset + (out.results as unknown[]).length;
  out.total = items.length;
  delete out.next_cursor;
  delete out.omitted;
  if (next < items.length) {
    out.next_cursor = `${prefix}${next}`;
    out.omitted = { reason: "page", remaining: items.length - next, how: "call again with this next_cursor" };
  }
  return out;
}
