import { useEffect, useRef } from "react";

interface Props {
  query: string;
  onQuery: (query: string) => void;
  matches: number;
  total: number;
}

/** Narrows the reference by tool or argument name as the reader types; "/" focuses it. */
export function ToolSearch({ query, onQuery, matches, total }: Props) {
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
    <div role="search" className="mb-8">
      <label htmlFor="tool-search" className="mb-2 block text-[14px] font-semibold">
        Find a tool or an argument
      </label>
      <input
        ref={input}
        id="tool-search"
        type="search"
        value={query}
        onChange={(event) => {
          onQuery(event.target.value);
        }}
        placeholder="fill, variable, render…   ( / )"
        autoComplete="off"
        className="w-full rounded-xl border border-line bg-surface px-4 py-3 font-mono text-[15px] text-text placeholder:text-muted"
      />
      <p aria-live="polite" className="mt-2 text-[13.5px] text-muted">
        {query ? `${matches} of ${total} tools` : `${total} tools`}
      </p>
    </div>
  );
}
