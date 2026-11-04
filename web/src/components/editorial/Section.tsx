import type { ReactNode } from "react";
import { useInView } from "~/lib/inView";
import { cn } from "~/lib/cn";

interface Props {
  id: string;
  /** "01", "02"…: the page's index, as a micro label. */
  index: string;
  label: string;
  tone?: "paper" | "sunk" | "ink" | undefined;
  className?: string | undefined;
  children: ReactNode;
}

/** A numbered section on the visible grid; its entry motion starts once in view. */
export function Section({ id, index, label, tone = "paper", className, children }: Props) {
  const { ref, seen } = useInView<HTMLElement>(0.15);
  return (
    <section
      ref={ref}
      id={id}
      aria-labelledby={`${id}-title`}
      className={cn(
        "grid-lines relative px-4 py-24 lg:px-0 lg:py-32",
        tone === "ink" && "tone-ink",
        tone === "sunk" && "bg-sunk",
        seen && "in-view",
        className,
      )}
    >
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <p className="micro col-span-4 text-muted lg:col-start-2">
          {index} — {label}
        </p>
      </div>
      {children}
    </section>
  );
}
