import { faq } from "~/content/faq";
import { Rich } from "../Rich";
import { FrameName } from "../figma";

/** Questions as native details: readable and operable without a script. */
export function Questions() {
  return (
    <section id="faq" aria-labelledby="faq-title" className="grid-lines px-4 py-24 lg:px-0 lg:py-32">
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-3 lg:col-start-2">
          <FrameName>Questions</FrameName>
          <h2
            id="faq-title"
            className="mt-6 text-[36px] leading-[1.1] font-normal tracking-[-0.03em] sm:text-[48px]"
          >
            Asked before installing.
          </h2>
        </div>
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
    </section>
  );
}
