import type { CSSProperties } from "react";
import type { SessionSpec } from "~/content/sessions";
import { cn } from "~/lib/cn";
import { AuditReport, OrdersList, PricingPlans } from "../live/Canvases";
import { OrderCard, type Mode } from "../live/OrderCard";
import type { Recording, Step } from "./types";

/** The grafta calls a recording had made by turn `at` (inclusive), and the beat they reached. */
export function progress(spec: SessionSpec, recording: Recording, at: number) {
  const done: Step[] = [];
  let beat = -1;
  recording.turns.slice(0, at + 1).forEach((turn) => {
    if (turn.kind !== "tools" || turn.other) return;
    done.push(...turn.calls);
    beat = Math.max(
      beat,
      spec.beats.findIndex((b) => b.id === turn.beat),
    );
  });
  return { done, beat };
}

function parse(text: string): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(text);
    return value && typeof value === "object" ? (value as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

/** "Accent soft" → "--t-accent-soft": the Theme's variables as OrderCard reads them. */
const cssVar = (name: string) => `--t-${name.trim().toLowerCase().replace(/\s+/g, "-")}`;

/** What set_variable gave a variable in one mode, in either shape the tool takes. */
function valueIn(step: Step, mode: string): string | undefined {
  const values = step.args.values as Record<string, unknown> | undefined;
  if (values && typeof values[mode] === "string") return values[mode];
  if (step.args.mode === mode && typeof step.args.value === "string") return step.args.value;
  return undefined;
}

/** The Theme's High contrast values a recording set, by the variable's name, up to now. */
export function themeValues(calls: Step[], mode = "High contrast"): Record<string, string> {
  const out: Record<string, string> = {};
  for (const step of calls) {
    if (step.tool !== "set_variable") continue;
    const value = valueIn(step, mode);
    const name = typeof step.args.name === "string" ? step.args.name : step.args.id;
    if (value && typeof name === "string" && !/^\d+:\d+$/.test(name)) out[name] = value;
  }
  return out;
}

function Figure({ label, children, ring }: { label: string; children: React.ReactNode; ring?: boolean }) {
  return (
    <figure className="flex flex-col gap-2">
      <figcaption className="micro text-muted">{label}</figcaption>
      <div className={cn("rounded-2xl", ring && "shadow-[0_0_0_2px_var(--sap)]")}>{children}</div>
    </figure>
  );
}

/**
 * The Figma file beside the conversation, at the beat it has reached:
 * drawn from components, every value the model chose read from
 * the calls it made. The audit's figures are the input file's own.
 */
export function Design({
  spec,
  recording,
  at,
  compact = false,
}: {
  spec: SessionSpec;
  recording: Recording;
  at: number;
  compact?: boolean;
}) {
  const { done, beat } = progress(spec, recording, at);
  const reached = (id: string) => beat >= spec.beats.findIndex((b) => b.id === id) && beat >= 0;
  const all = recording.turns.flatMap((t) => (t.kind === "tools" && !t.other ? t.calls : []));

  if (spec.id === "image") {
    // The picture the person gave, beside the file as grafta last drew it (render_node's own picture).
    const drawn = done.filter((s) => s.picture).at(-1)?.picture;
    const box = compact ? "w-[150px]" : "w-[220px]";
    return (
      <div className="flex items-start justify-center gap-5">
        <Figure label="poster.png · the picture">
          <img
            fetchPriority="low"
            decoding="async"
            src={`${import.meta.env.BASE_URL}showcase/poster.webp`}
            width={440}
            height={550}
            alt="The poster the person gave: Forma 27, design days, 14–15 May"
            className={cn(box, "h-auto rounded-xl border border-black/10")}
          />
        </Figure>
        <Figure
          label={drawn ? "The Figma file · as grafta drew it" : "The Figma file · being built"}
          ring={!!drawn}
        >
          {drawn ? (
            <img
              fetchPriority="low"
              decoding="async"
              src={`${import.meta.env.BASE_URL}${drawn.src}`}
              width={drawn.width}
              height={drawn.height}
              alt={drawn.alt}
              className={cn(box, "h-auto rounded-xl")}
            />
          ) : (
            <div
              className={cn(
                box,
                "grid aspect-[4/5] place-items-center rounded-xl border border-dashed border-line p-4 text-center text-[13px] text-muted",
              )}
            >
              {reached("text")
                ? "Text and shapes as layers…"
                : reached("read")
                  ? "A frame the size of the picture…"
                  : "Not started"}
            </div>
          )}
        </Figure>
      </div>
    );
  }

  if (spec.id === "theme") {
    const hc = themeValues(done);
    const total = Object.keys(themeValues(all)).length;
    const filled = Object.keys(hc).length;
    const style = Object.fromEntries(Object.entries(hc).map(([k, v]) => [cssVar(k), v])) as CSSProperties;
    const rendered = reached("render");
    const cards: { m: Mode; label: string; shown: boolean }[] = [
      { m: "light", label: "Light", shown: true },
      { m: "dark", label: "Dark", shown: reached("read") },
      {
        m: "hc",
        label: `High contrast${filled < total ? ` · ${filled} of ${total} colours` : ""}`,
        shown: reached("mode"),
      },
    ];
    const shown = cards.filter((c) => c.shown);
    const visible = compact ? shown.slice(-1) : shown;
    return (
      <div className={cn("flex flex-wrap justify-center gap-4", !compact && "w-[500px]")}>
        {visible.map(({ m, label }) => (
          <Figure key={m} label={`${label}${rendered ? " · rendered" : ""}`} ring={rendered}>
            <OrderCard
              mode={m}
              style={m === "hc" ? style : undefined}
              className={cn("reveal", m === "hc" && filled < total && "opacity-60")}
            />
          </Figure>
        ))}
      </div>
    );
  }

  if (spec.id === "audit") {
    return (
      <div className="flex flex-wrap items-start justify-center gap-6">
        <OrderCard mode="light" />
        <AuditReport found={new Set(done.map((s) => s.tool))} />
      </div>
    );
  }

  if (spec.id === "contrast") {
    const checks = done.filter((s) => s.tool === "check_contrast").map((s) => parse(s.result));
    const last = checks.at(-1);
    const fixed = reached("fix");
    const fix = done.find((s) => s.tool === "set_properties");
    const paint = (fix?.args.properties as { fills?: { color?: string; variable?: string }[] } | undefined)
      ?.fills?.[0];
    const noteColor = paint?.variable ? `var(${cssVar(paint.variable)})` : paint?.color;
    const failed = typeof last?.failed === "number" ? last.failed : undefined;
    const checked = typeof last?.checked === "number" ? last.checked : undefined;
    return (
      <div className="flex flex-col items-center gap-5">
        <OrderCard mode="light" noteFails={!fixed} noteColor={noteColor} />
        {failed !== undefined && checked !== undefined && (
          <p
            className={cn(
              "reveal micro rounded-full border px-3 py-1.5",
              failed === 0 ? "border-deep-sap text-deep-sap" : "border-[#b5401f] text-[#b5401f]",
            )}
          >
            {failed === 0
              ? `check_contrast: ${checked} of ${checked} pass`
              : `check_contrast: ${failed} of ${checked} fail`}
          </p>
        )}
      </div>
    );
  }

  if (spec.id === "data") {
    const component = reached("component");
    const template = done.find((s) => s.tool === "fill_template");
    const rows = Array.isArray(template?.args.rows)
      ? (template.args.rows as Record<string, unknown>[]).map((row) =>
          Object.values(row).filter((v): v is string => typeof v === "string"),
        )
      : Array.from({ length: done.filter((s) => s.tool === "create_instance").length }, () => [] as string[]);
    return (
      <div className="flex flex-wrap items-start justify-center gap-6">
        <Figure label={component ? "Card · component" : "Card · frame"}>
          <OrderCard mode="light" className={component ? "shadow-[0_0_0_1px_#9747FF]" : undefined} />
        </Figure>
        {reached("rows") && (
          <Figure label={`Orders · ${rows.length} instance${rows.length === 1 ? "" : "s"}`}>
            <OrdersList rows={rows} />
          </Figure>
        )}
      </div>
    );
  }

  // html: the snapshot's boxes as layers, once imported
  const imported = reached("import");
  return (
    <figure className="flex w-full flex-col items-center gap-3">
      <figcaption className="micro text-muted">
        {imported ? "Pricing · imported as layers" : "order.fig"}
      </figcaption>
      {imported ? (
        <PricingPlans />
      ) : (
        <div className="grid h-[180px] w-full max-w-[460px] place-items-center rounded-2xl border border-dashed border-line text-[14px] text-muted">
          pricing.json waits to be imported
        </div>
      )}
    </figure>
  );
}
