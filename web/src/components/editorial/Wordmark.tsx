import { cn } from "~/lib/cn";

/** The wordmark as structure: as wide as the page, its letters rising once. Decorative. */
export function Wordmark({ className }: { className?: string | undefined }) {
  return (
    <p
      aria-hidden="true"
      className={cn(
        "wordmark pointer-events-none font-bold whitespace-nowrap select-none",
        "text-[38.3vw] leading-[0.8] tracking-[-0.06em]",
        className,
      )}
    >
      {"grafta".split("").map((letter, i) => (
        <span key={i} style={{ "--i": i } as React.CSSProperties}>
          {letter}
        </span>
      ))}
    </p>
  );
}
