import type { ReactNode } from "react";
import { cn } from "~/lib/cn";

/**
 * Figma's selection box: a hairline in Sap, four handles, and the node's tag,
 * drawn around what it selects. Decoration: hidden from assistive technology.
 */
export function Selection({
  tag,
  below = false,
  className,
  children,
}: {
  tag: string;
  below?: boolean | undefined;
  className?: string | undefined;
  children: ReactNode;
}) {
  return (
    <div className={cn("relative", className)}>
      {children}
      <div
        aria-hidden="true"
        className="selection-box pointer-events-none absolute -inset-2 border border-sap"
      >
        {(["-top-1 -left-1", "-top-1 -right-1", "-bottom-1 -left-1", "-bottom-1 -right-1"] as const).map(
          (at) => (
            <span
              key={at}
              className={cn("selection-handle absolute size-[7px] border border-sap bg-white", at)}
            />
          ),
        )}
        <span
          className={cn(
            "absolute left-[-1px] rounded-[3px] bg-deep-sap px-1.5 py-px text-[12px] leading-4 font-medium text-white",
            below ? "top-full mt-2" : "-top-6",
          )}
        >
          {tag}
        </span>
      </div>
    </div>
  );
}
