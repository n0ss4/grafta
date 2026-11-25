import type { ReactNode } from "react";
import { cn } from "~/lib/cn";

/** The home page's width: wide, with the gutter every section shares. */
export function Wrap({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cn("mx-auto max-w-[80rem] px-4 lg:px-8", className)}>{children}</div>;
}

/**
 * One section of the home page: a hairline across the whole page above it, and
 * the same room above and below, so the sections read as bands.
 */
export function Band({
  id,
  label,
  labelledBy,
  padding = "py-[clamp(80px,10vw,136px)]",
  className,
  children,
}: {
  id: string;
  label?: string;
  labelledBy?: string;
  /** The room above and below; the same for every section unless one needs less. */
  padding?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section
      id={id}
      aria-label={label}
      aria-labelledby={labelledBy}
      className={cn("scroll-mt-16 border-t border-line", padding, className)}
    >
      {children}
    </section>
  );
}

/** The heading every section shares: large, set tight, the weight of the page's title. */
export const headingClass =
  "text-[length:clamp(34px,4vw,48px)] leading-[1.05] font-semibold tracking-[-0.03em] text-balance";

/**
 * A section's opening: its title, and under it what it says, the first
 * sentence in the text's colour and the rest quieter.
 */
export function Head({
  id,
  title,
  lead,
  rest,
  badge,
  center = false,
  children,
}: {
  id: string;
  title: ReactNode;
  lead: string;
  rest: string;
  badge?: ReactNode;
  center?: boolean;
  children?: ReactNode;
}) {
  return (
    <div className={cn("max-w-[44rem]", center && "mx-auto text-center")}>
      {badge}
      <h2 id={id} className={headingClass}>
        {title}
      </h2>
      <p className="mt-5 text-[length:clamp(17px,1.5vw,19px)] leading-[1.55] text-muted">
        <span className="text-text">{lead}</span> {rest}
      </p>
      {children}
    </div>
  );
}

/** "New", with its spark: a small label in a pill. */
export function NewBadge({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex h-8 items-center gap-1.5 rounded-full bg-surface pr-3.5 pl-2.5 text-[14px] font-[510] text-text ring-1 ring-line ring-inset",
        className,
      )}
    >
      <svg width="17" height="17" viewBox="0 0 24 24" aria-hidden="true" className="text-accent-text">
        <path d="M12 2.5l1.9 5.6 5.6 1.9-5.6 1.9L12 17.5l-1.9-5.6L4.5 10l5.6-1.9z" fill="currentColor" />
        <path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z" fill="currentColor" opacity="0.7" />
      </svg>
      New
    </span>
  );
}

/** A point made in a line, after a check in grafta's colour. */
export function Checks({
  items,
  className,
}: {
  items: readonly (readonly [string, string])[];
  className?: string;
}) {
  return (
    <ul className={cn("grid gap-x-8 gap-y-3.5 sm:grid-cols-2 lg:grid-cols-4", className)}>
      {items.map(([title, text]) => (
        <li key={title} className="flex gap-3 text-[15px] leading-relaxed">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className="mt-[5px] flex-none text-accent-text"
          >
            <path d="M20 6 9 17l-5-5" />
          </svg>
          <span>
            <b className="font-[510] text-text">{title}.</b> <span className="text-muted">{text}</span>
          </span>
        </li>
      ))}
    </ul>
  );
}
