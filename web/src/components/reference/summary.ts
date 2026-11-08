/** How long a closed tool's line may be, in characters. */
export const SUMMARY_MAX = 110;

/**
 * A tool's line when closed: its description's first sentence, cut at its first
 * colon or, failing that, at a word, when longer than SUMMARY_MAX. Always the
 * server's own words, a prefix of the description; the whole is shown open.
 */
export function summary(description: string): string {
  const text = description.trim();
  // a sentence ends at . ! or ? before a space and a capital, a backtick or the end;
  // "e.g." and "i.e." do not end one
  const end = /(?<!\be\.g|\bi\.e)[.!?](?=\s+[A-Z`]|\s*$)/.exec(text);
  const first = end ? text.slice(0, end.index + 1) : text;
  if (first.length <= SUMMARY_MAX) return first;
  const colon = first.indexOf(": ");
  if (colon >= 20 && colon <= SUMMARY_MAX) return first.slice(0, colon);
  const cut = first.lastIndexOf(" ", SUMMARY_MAX - 1);
  return `${first.slice(0, cut).replace(/[,;(]$/, "")}…`;
}
