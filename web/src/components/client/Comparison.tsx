import { cn } from "~/lib/cn";
import { Check } from "../editorial/icons";
import { Section } from "../editorial/Section";
import { clientName, type Recording } from "./types";
import { Reply } from "./view";

const title = "text-[36px] leading-[1.1] font-normal tracking-[-0.03em] sm:text-[48px]";

/**
 * 01: the same request sent twice in the same client, without grafta and
 * with it, both recorded. Each side ends with what
 * the person holds; that line is the site's, said as such.
 */
export function Comparison({ without, withGrafta }: { without: Recording; withGrafta: Recording }) {
  const prompt = without.turns[0]?.kind === "user" ? without.turns[0].text : "";
  const sides = [
    {
      key: "without",
      heading: "Without grafta",
      recording: without,
      words: without.turns.filter((t) => t.kind === "reply").map((t) => t.text),
      holds: "A list of what it would need, and the file untouched.",
      good: false,
    },
    {
      key: "with",
      heading: "With grafta",
      recording: withGrafta,
      words: withGrafta.turns
        .filter((t) => t.kind === "reply")
        .slice(-1)
        .map((t) => t.text),
      holds: withGrafta.file ? `${withGrafta.file}, a new file that opens in Figma.` : "The change, made.",
      good: true,
    },
  ];
  return (
    <Section id="compare" index="01" label="Without grafta, with grafta">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <h2 id="compare-title" className={title}>
            The same request, twice.
          </h2>
          <p className="mt-6 max-w-[620px] text-[17px] leading-relaxed text-muted">
            Sent in {clientName[without.client]}, once with grafta not installed and once with it. Both
            recorded; the replies are the model's own.
          </p>
        </div>
      </div>
      <div className="mt-12 grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 grid gap-6 lg:col-span-6 lg:col-start-2 lg:grid-cols-2">
          {sides.map((side) => (
            <article
              key={side.key}
              aria-labelledby={`compare-${side.key}`}
              className={cn(
                "flex flex-col gap-4 rounded-2xl border p-6 sm:p-7",
                side.good ? "border-accent/60 bg-surface" : "border-line bg-bg",
              )}
            >
              <div>
                <h3 id={`compare-${side.key}`} className="text-[22px] font-medium">
                  {side.heading}
                </h3>
                <p className="mt-1 text-[12px] font-medium text-muted">
                  {clientName[side.recording.client]} {side.recording.clientVersion}, {side.recording.model},{" "}
                  {side.recording.recordedOn}
                </p>
              </div>
              <p className="rounded-xl bg-sunk p-3 text-[14px] leading-5">{prompt}</p>
              <div className="flex-1 overflow-hidden text-[15px] leading-6 [overflow-wrap:anywhere]">
                {side.words.map((words, i) => (
                  <div key={i} className={i ? "mt-3" : undefined}>
                    <Reply text={words} />
                  </div>
                ))}
              </div>
              <p
                className={cn(
                  "flex items-start gap-2.5 rounded-xl p-3 text-[15px] font-semibold",
                  side.good ? "bg-soft" : "bg-sunk",
                )}
              >
                {side.good ? (
                  <Check size={16} className="mt-0.5 flex-none text-accent-text" />
                ) : (
                  <span aria-hidden className="mt-px flex-none text-muted">
                    ✕
                  </span>
                )}
                <span>
                  <span className="font-medium text-muted">You hold: </span>
                  {side.holds}
                </span>
              </p>
            </article>
          ))}
        </div>
        <p className="micro col-span-4 mt-6 text-muted lg:col-start-2">
          “You hold” is the site's summary; everything above it is as recorded
        </p>
      </div>
    </Section>
  );
}
