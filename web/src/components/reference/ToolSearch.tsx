import { useEffect, useRef } from "react";

interface Props {
  query: string;
  onQuery: (query: string) => void;
  matches: number;
  total: number;
  allOpen: boolean;
  onOpenAll: (open: boolean) => void;
}

/**
 * Sticky under the nav: narrows the reference by a tool's name, its arguments or
 * what it does, as the reader types ("/" focuses it); says how many; opens or
 * closes every tool.
 */
export function ToolSearch({ query, onQuery, matches, total, allOpen, onOpenAll }: Props) {
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      if (event.key === "/" && target?.tagName !== "INPUT" && target?.tagName !== "TEXTAREA") {
        event.preventDefault();
        input.current?.focus();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  return (
    <div
      role="search"
      className="sticky top-16 z-30 -mx-4 mt-8 mb-10 border-b border-line bg-bg/90 px-4 py-3 backdrop-blur-md lg:mx-0 lg:px-0"
    >
      <label htmlFor="tool-search" className="sr-only">
        Find a tool by its name, an argument, or what it does
      </label>
      <div className="flex items-center gap-3">
        <input
          ref={input}
          id="tool-search"
          type="search"
          value={query}
          onChange={(event) => {
            onQuery(event.target.value);
          }}
          placeholder="Find a tool…"
          autoComplete="off"
          className="min-w-0 flex-1 rounded-xl border border-line bg-surface px-4 py-2.5 text-[15px] text-text placeholder:text-muted"
        />
        <p aria-live="polite" className="text-[13.5px] whitespace-nowrap text-muted">
          {query ? `${matches} of ${total} tools` : `${total} tools`}
        </p>
        <button
          type="button"
          onClick={() => {
            onOpenAll(!allOpen);
          }}
          className="cursor-pointer rounded-lg border border-line px-3 py-2 text-[14px] whitespace-nowrap text-text hover:border-text"
        >
          {allOpen ? "Close all" : "Open all"}
        </button>
      </div>
    </div>
  );
}
