import type { ReactNode } from "react";
import { cn } from "~/lib/cn";

/** Decorative Figma concepts, redrawn in grafta's colours. */
export function Selection({
  label,
  size,
  className,
  children,
}: {
  label?: string;
  size?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn("relative", className)}>
      {children}
      <span
        aria-hidden="true"
        className="pointer-events-none absolute -inset-x-4 -inset-y-3 border border-sap"
      >
        {(["-top-1 -left-1", "-top-1 -right-1", "-bottom-1 -left-1", "-bottom-1 -right-1"] as const).map(
          (at) => (
            <span key={at} className={cn("absolute size-2 border border-sap bg-white", at)} />
          ),
        )}
        {label && (
          <span className="absolute bottom-full left-[-1px] mb-2 rounded-[3px] bg-deep-sap px-1.5 py-px text-[11px] leading-4 font-medium whitespace-nowrap text-white">
            {label}
          </span>
        )}
        {size && (
          <span className="absolute top-full left-1/2 mt-2 hidden -translate-x-1/2 rounded bg-deep-sap px-1.5 py-px font-mono text-[10px] leading-4 whitespace-nowrap text-white xl:block">
            {size}
          </span>
        )}
      </span>
    </div>
  );
}

export function FrameName({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn("inline-flex items-center gap-1.5 font-mono text-[11px] text-muted", className)}
    >
      <span className="grid size-3 place-items-center border border-current text-[8px]">#</span>
      {children}
    </span>
  );
}

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

export function Measure({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn("inline-flex items-center gap-1 font-mono text-[10px] text-rust", className)}
    >
      <i className="h-px w-3 bg-current" />
      {children}
      <i className="h-px w-3 bg-current" />
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
