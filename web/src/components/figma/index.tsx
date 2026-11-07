import { cn } from "~/lib/cn";

/** Decorative Figma concepts, redrawn in grafta's colours. */
export function ComponentMark({ className }: { className?: string }) {
  return (
    <span aria-hidden="true" className={cn("inline-grid size-4 rotate-45 grid-cols-2 gap-px", className)}>
      <i className="border border-current" />
      <i className="border border-current" />
      <i className="border border-current" />
      <i className="border border-current" />
    </span>
  );
}

export function Cursor({ name = "Your agent", className }: { name?: string; className?: string }) {
  return (
    <span aria-hidden="true" className={cn("pointer-events-none absolute z-10 text-[#B5532F]", className)}>
      <svg width="16" height="22" viewBox="0 0 16 22" fill="none">
        <path
          d="M1 1.5v16l4.4-4.2 3.1 7 3-1.4-3.1-6.8H15L1 1.5Z"
          fill="currentColor"
          stroke="white"
          strokeWidth="1.5"
        />
      </svg>
      <span className="absolute top-5 left-3 rounded bg-[#B5532F] px-2 py-0.5 text-[11px] leading-4 whitespace-nowrap text-white">
        {name}
      </span>
    </span>
  );
}
