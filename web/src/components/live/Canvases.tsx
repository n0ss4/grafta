import { cn } from "~/lib/cn";
import { Check } from "../editorial/icons";
import { OrderRow } from "./OrderCard";

/** The Theme's nine colours, as list_variables answered for order.fig (the audit session). */
export const themeTokens = [
  ["Background", "#F6F4EE", "#0D1117"],
  ["Surface", "#FFFFFF", "#161B22"],
  ["Ink", "#15171C", "#F6F4EE"],
  ["Muted", "#6B6F76", "#9DA3AB"],
  ["Border", "#E6E2D8", "#30363D"],
  ["Accent", "#13704B", "#C6F26B"],
  ["Accent soft", "#E3F4EA", "#1C2B23"],
  ["On accent", "#FFFFFF", "#15171C"],
  ["Track", "#ECE8DE", "#21262D"],
] as const;

/** The audit, as its calls add to it: each line appears with the call that found it. */
export function AuditReport({ found }: { found: Set<string> }) {
  const row = "flex items-baseline justify-between gap-4 border-t border-line py-2.5 text-[14px]";
  return (
    <div className="w-full max-w-[360px] rounded-2xl border border-line bg-surface p-5 text-left">
      <p className="micro text-muted">Audit · order.fig</p>
      <dl className="mt-4">
        {found.has("summarize") && (
          <div className={cn(row, "reveal")}>
            <dt className="text-muted">Nodes</dt>
            <dd>23, on 1 page</dd>
          </div>
        )}
        {found.has("list_variables") && (
          <div className={cn(row, "reveal flex-col items-stretch")}>
            <span className="flex justify-between">
              <dt className="text-muted">Tokens</dt>
              <dd>9 colours · Light, Dark</dd>
            </span>
            <span className="mt-2 flex gap-1.5" aria-hidden="true">
              {themeTokens.map(([name, light, dark]) => (
                <span
                  key={name}
                  title={name}
                  className="flex h-6 flex-1 overflow-hidden rounded border border-line"
                >
                  <span className="flex-1" style={{ background: light }} />
                  <span className="flex-1" style={{ background: dark }} />
                </span>
              ))}
            </span>
          </div>
        )}
        {found.has("list_components") && (
          <div className={cn(row, "reveal")}>
            <dt className="text-muted">Components</dt>
            <dd>0</dd>
          </div>
        )}
        {found.has("list_styles") && (
          <div className={cn(row, "reveal")}>
            <dt className="text-muted">Styles</dt>
            <dd>0</dd>
          </div>
        )}
        {found.has("read_node") && (
          <div className={cn(row, "reveal")}>
            <dt className="text-muted">“Your order”</dt>
            <dd className="inline-flex items-center gap-1.5 text-accent-text">
              <Check size={14} /> fill bound to Ink
            </dd>
          </div>
        )}
      </dl>
    </div>
  );
}

/**
 * The list the data session builds: one card per row it placed, each showing
 * the row's own values as the recording sent them. A row with no values
 * (an instance placed by hand) shows the card as it is.
 */
export function OrdersList({ rows }: { rows: string[][] }) {
  return (
    <div
      data-node="FRAME Orders"
      className="mode-light flex w-[260px] flex-col gap-3 rounded-2xl bg-[var(--t-background)] p-4"
    >
      {rows.map(([number, ...rest], i) => (
        <OrderRow
          key={i}
          number={number}
          eta={rest.length ? rest.join(" · ") : undefined}
          className="reveal"
        />
      ))}
    </div>
  );
}

/** The two plans import_layout wrote from the pricing page's snapshot (docs/showcase/pricing.html). */
export function PricingPlans() {
  const plans = [
    { name: "Starter", price: "$0", note: "One file at a time, offline.", cta: "Get started", pro: false },
    { name: "Team", price: "$12", note: "Shared tokens and components.", cta: "Start a trial", pro: true },
  ];
  return (
    <div data-node="FRAME plans" className="flex w-full max-w-[460px] gap-3 rounded-2xl bg-paper p-4">
      {plans.map((plan) => (
        <div
          key={plan.name}
          data-node="FRAME plan"
          className={cn(
            "reveal flex flex-1 flex-col gap-2.5 rounded-[16px] border p-4 text-left",
            plan.pro ? "border-ink bg-ink" : "border-[#E6E2D8] bg-white",
          )}
        >
          <p className={cn("text-[12px] font-semibold", plan.pro ? "text-bud" : "text-[#6B6F76]")}>
            {plan.name}
          </p>
          <p
            className={cn(
              "text-[30px] leading-none font-bold tracking-[-0.03em]",
              plan.pro ? "text-paper" : "text-ink",
            )}
          >
            {plan.price}
          </p>
          <p className={cn("text-[11px]", plan.pro ? "text-[#9DA3AB]" : "text-[#6B6F76]")}>{plan.note}</p>
          <span
            className={cn(
              "rounded-[10px] py-2 text-center text-[12px] font-semibold text-ink",
              plan.pro ? "bg-bud" : "bg-[#ECE8DE]",
            )}
          >
            {plan.cta}
          </span>
        </div>
      ))}
    </div>
  );
}
