import { faq } from "~/content/faq";
import { Rich } from "../Rich";
import { Band, headingClass, Wrap } from "./Section";

/** Questions as native details: readable and operable without a script. */
export function Questions() {
  return (
    <Band id="faq" labelledBy="faq-title">
      <Wrap className="grid gap-10 lg:grid-cols-2 lg:gap-16">
        <div>
          <h2 id="faq-title" className={headingClass}>
            Asked before
            <br />
            installing
          </h2>
        </div>
        <div>
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
      </Wrap>
    </Band>
  );
}
