import { comparison } from "~/content/editorial";
import { FigmaMark } from "../editorial/icons";

const cards = [
  {
    name: "grafta",
    icon: null,
    key: "grafta" as const,
    lead: "For files on your disk, repeated edits, and working with no Figma seat.",
  },
  {
    name: "Figma's MCP server",
    icon: <FigmaMark size={16} />,
    key: "figma" as const,
    lead: "For working live in a file open in Figma.",
  },
];

export function Difference() {
  return (
    <section id="difference" aria-labelledby="difference-title" className="px-4 py-24 lg:px-0 lg:py-32">
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <h2 id="difference-title" className="text-[36px] font-normal tracking-[-0.03em] sm:text-[48px]">
            grafta or Figma's MCP server?
          </h2>
          <p className="mt-3 max-w-[680px] text-[17px] leading-relaxed text-muted">
            Choose by where the work needs to happen. They do different jobs.
          </p>
          <div className="mt-12 grid gap-5 lg:grid-cols-2">
            {cards.map((card) => (
              <article key={card.name} className="rounded-2xl border border-line bg-surface p-6">
                <h3 className="flex items-center gap-2 text-[24px] font-normal text-accent-text">
                  {card.icon}
                  {card.name}
                </h3>
                <p className="mt-2 text-[14px] text-muted">{card.lead}</p>
                <dl className="mt-6">
                  {comparison.map((row) => (
                    <div
                      key={row.aspect}
                      className="grid grid-cols-[90px_1fr] gap-4 border-t border-line py-3 text-[14px]"
                    >
                      <dt className="text-muted">{row.aspect}</dt>
                      <dd>{row[card.key]}</dd>
                    </div>
                  ))}
                </dl>
              </article>
            ))}
          </div>
          <p className="mt-6 font-mono text-[11px] text-muted">
            Figma Help Center and developer documentation · 30 October 2026
          </p>
        </div>
      </div>
    </section>
  );
}
