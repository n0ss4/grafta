import { faq } from "~/content/faq";
import { Rich } from "../Rich";
import { Section } from "../editorial/Section";

/** 08: questions, as details: open without a script. */
export function Questions() {
  return (
    <Section id="faq" index="08" label="Questions">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <h2
          id="faq-title"
          className="col-span-4 text-[36px] leading-[1.1] font-normal tracking-[-0.03em] sm:text-[48px] lg:col-span-3 lg:col-start-2"
        >
          Asked before installing.
        </h2>
        <div className="col-span-4 mt-10 lg:col-span-3 lg:col-start-5 lg:mt-0">
          {faq.map((entry) => (
            <details key={entry.question} className="group border-t border-line py-5 last:border-b">
              <summary className="flex cursor-pointer list-none items-baseline justify-between gap-6 text-[17px] font-medium marker:hidden">
                {entry.question}
                <span aria-hidden="true" className="text-muted transition-transform group-open:rotate-45">
                  +
                </span>
              </summary>
              <p className="mt-3 text-[15px] leading-relaxed text-muted">
                <Rich text={entry.answer} />
              </p>
            </details>
          ))}
        </div>
      </div>
    </Section>
  );
}
