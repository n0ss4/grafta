import { cn } from "~/lib/cn";
import { ArrowRight, Package } from "../editorial/icons";

export type Mode = "light" | "dark" | "hc";
export type Part = "title" | "meta" | "card" | "button" | "note";

const modeClass: Record<Mode, string> = { light: "mode-light", dark: "mode-dark", hc: "mode-hc" };

/**
 * The order screen grafta edits in its theme session, as the Figma file's
 * `Order card` component draws it: the same layers (named in data-node), every
 * colour one of the Theme's variables, resolved in the mode given.
 */
export function OrderCard({
  mode,
  hidden = [],
  noteFails = false,
  noteColor,
  style,
  className,
}: {
  mode: Mode;
  hidden?: Part[] | undefined;
  /** The note in the checkout file's colour, #B9B6AE: 1.84:1 on Background, as check_contrast found it. */
  noteFails?: boolean | undefined;
  /** The note's colour once fixed, as the recording set it; the mode's Muted otherwise. */
  noteColor?: string | undefined;
  /** The Theme's values for this mode, as a recording set them (`--t-accent` …), over the mode's own. */
  style?: React.CSSProperties | undefined;
  className?: string | undefined;
}) {
  const show = (part: Part) => !hidden.includes(part);
  return (
    <div
      data-node="FRAME Order"
      style={style}
      className={cn(
        modeClass[mode],
        "flex w-[232px] flex-col gap-3 rounded-2xl bg-[var(--t-background)] p-4 text-left transition-colors duration-[400ms]",
        className,
      )}
    >
      <p
        data-node="TEXT title"
        className={cn("text-[18px] leading-6 font-bold text-[var(--t-ink)]", !show("title") && "invisible")}
      >
        Your order
      </p>
      <p
        data-node="TEXT meta"
        className={cn("text-[11px] leading-4 text-[var(--t-muted)]", !show("meta") && "invisible")}
      >
        Placed 2 Oct · 3 items
      </p>
      <OrderRow className={cn(!show("card") && "invisible")} />
      <span
        data-node="FRAME Button"
        className={cn(
          "flex items-center justify-center gap-1.5 rounded-[10px] bg-[var(--t-accent)] p-2.5 text-[12px] leading-4 font-bold text-[var(--t-on-accent)]",
          !show("button") && "invisible",
        )}
      >
        <span data-node="TEXT label">Track order</span> <ArrowRight size={14} strokeWidth={2} />
      </span>
      <p
        data-node="TEXT note"
        className={cn(
          "text-[10px] leading-[14px] transition-colors duration-500",
          noteFails ? "text-[#B9B6AE]" : "text-[var(--t-muted)]",
          !show("note") && "invisible",
        )}
        style={!noteFails && noteColor ? { color: noteColor } : undefined}
      >
        Delivered to the door · no signature
      </p>
    </div>
  );
}

/** The order card's Card frame alone: the component fill_template places once per row. */
export function OrderRow({
  number = "Order #4821",
  eta = "Arriving tomorrow, 9–12",
  className,
}: {
  number?: string | undefined;
  eta?: string | undefined;
  className?: string | undefined;
}) {
  return (
    <div
      data-node="FRAME Card"
      className={cn(
        "flex flex-col gap-2.5 rounded-[14px] border border-[var(--t-border)] bg-[var(--t-surface)] p-3 text-left",
        className,
      )}
    >
      <div className="flex items-center gap-2.5">
        <span
          data-node="FRAME Icon"
          className="grid size-[30px] place-items-center rounded-[10px] bg-[var(--t-accent-soft)] text-[var(--t-accent)]"
        >
          <Package size={16} strokeWidth={2} />
        </span>
        <span className="flex flex-col">
          <span className="text-[12px] leading-4 font-bold text-[var(--t-ink)]">{number}</span>
          <span className="text-[10px] leading-[14px] text-[var(--t-muted)]">{eta}</span>
        </span>
      </div>
      <span data-node="FRAME Progress" className="h-1.5 rounded-full bg-[var(--t-track)]">
        <span className="block h-1.5 w-[70%] rounded-full bg-[var(--t-accent)]" />
      </span>
      <span className="flex justify-between text-[9px] leading-3 font-medium text-[var(--t-muted)]">
        <span>Packed</span>
        <span>Shipped</span>
        <span className="font-bold text-[var(--t-accent)]">Out for delivery</span>
      </span>
    </div>
  );
}
