import { asks } from "~/content/editorial";
import { Check } from "../editorial/icons";
import { ComponentMark, FrameName } from "../figma";

export function Ask() {
  return (
    <section id="ask" aria-labelledby="ask-title" className="grid-lines px-4 py-24 lg:px-0 lg:py-32">
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <FrameName>What you can ask</FrameName>
          <h2 id="ask-title" className="mt-6 text-[36px] font-normal tracking-[-0.03em] sm:text-[48px]">
            Plain words. Real files back.
          </h2>
          <p className="mt-3 max-w-[660px] text-[17px] leading-relaxed text-muted">
            Each request ends in a new file for you to open in Figma, with layers you can keep editing. The
            pictures show the kind of result; their figures are examples.
          </p>
          <div className="mt-14 space-y-16">
            {asks.map((group) => (
              <section key={group.who} aria-labelledby={`ask-${group.who.toLowerCase()}`}>
                <h3
                  id={`ask-${group.who.toLowerCase()}`}
                  className="flex items-center gap-2 border-t border-text pt-4 text-[15px] font-medium"
                >
                  <ComponentMark className="text-accent-text" /> For {group.who.toLowerCase()}
                </h3>
                <div className="mt-6 grid gap-6 md:grid-cols-2">
                  {group.requests.map((request) => (
                    <article
                      key={request.prompt}
                      className="overflow-hidden rounded-2xl border border-line bg-surface"
                    >
                      <RequestVisual image={request.image} alt={request.alt} />
                      <p className="p-5 text-[17px] leading-relaxed">“{request.prompt}”</p>
                    </article>
                  ))}
                </div>
              </section>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

function RequestVisual({ image, alt }: { image: string; alt: string }) {
  const shell = "aspect-[16/9] w-full border-b border-line bg-sunk p-5 sm:p-7";
  if (image.includes("theme-")) {
    return (
      <div role="img" aria-label={alt} className={shell}>
        <div className="mx-auto h-full max-w-[430px] overflow-hidden rounded-lg border border-line bg-surface shadow-sm">
          <p className="border-b border-line px-4 py-2 font-mono text-[9px] text-muted">Variables · Brand</p>
          <div className="grid grid-cols-[1fr_repeat(3,64px)] text-[9px] sm:grid-cols-[1fr_repeat(3,82px)] sm:text-[10px]">
            {[
              ["", "Light", "Dark", "Contrast"],
              ["Background", "#F6F4EE", "#0D1117", "#000000"],
              ["Text", "#15171C", "#F6F4EE", "#FFFFFF"],
              ["Accent", "#13704B", "#C6F26B", "#FFE14D"],
            ].map((row, index) =>
              row.map((cell) => (
                <span
                  key={`${index}-${cell}`}
                  className={`border-b border-line px-2 py-2 ${index === 0 ? "font-mono text-muted" : index > 0 && cell !== row[0] ? "font-mono" : ""}`}
                >
                  {cell}
                </span>
              )),
            )}
          </div>
        </div>
      </div>
    );
  }
  if (image.includes("contrast-")) {
    return (
      <div role="img" aria-label={alt} className={`${shell} grid place-items-center`}>
        <div className="w-full max-w-[400px] rounded-xl border border-line bg-surface p-5 shadow-sm">
          <div className="flex items-end justify-between">
            <span>
              <span className="block font-mono text-[9px] text-muted">WCAG 2.2 AA</span>
              <strong className="mt-1 block text-[28px] font-normal">18 / 18 pass</strong>
            </span>
            <span className="grid size-10 place-items-center rounded-full bg-soft text-accent-text">
              <Check size={20} />
            </span>
          </div>
          <div className="mt-4 grid grid-cols-6 gap-1.5" aria-hidden="true">
            {Array.from({ length: 18 }, (_, index) => (
              <i key={index} className="h-2 rounded-full bg-accent" />
            ))}
          </div>
          <p className="mt-3 font-mono text-[9px] text-muted">
            0 failed · 0 unmeasured · checked on rendered backgrounds
          </p>
        </div>
      </div>
    );
  }
  if (image.includes("audit-")) {
    return (
      <div role="img" aria-label={alt} className={`${shell} grid place-items-center`}>
        <div className="grid w-full max-w-[440px] grid-cols-3 gap-px overflow-hidden rounded-xl border border-line bg-line shadow-sm">
          {[
            ["Pages", "12"],
            ["Components", "84"],
            ["Variables", "36"],
            ["Unbound colours", "7"],
            ["Detached instances", "3"],
            ["Unused styles", "11"],
          ].map(([label, value]) => (
            <div key={label} className="bg-surface p-3 sm:p-4">
              <strong className="block text-[22px] font-normal">{value}</strong>
              <span className="mt-1 block text-[9px] leading-tight text-muted sm:text-[10px]">{label}</span>
            </div>
          ))}
        </div>
      </div>
    );
  }
  if (image.includes("data-")) {
    return (
      <div role="img" aria-label={alt} className={`${shell} grid place-items-center`}>
        <div className="flex w-full max-w-[440px] items-center gap-4">
          <div className="flex-1 overflow-hidden rounded-lg border border-line bg-surface font-mono text-[8px] sm:text-[9px]">
            <p className="bg-code px-3 py-2 text-muted">products.csv</p>
            {["Noma lamp, €129", "Arc chair, €240", "Fold table, €315"].map((row) => (
              <p key={row} className="border-t border-line px-3 py-2">
                {row}
              </p>
            ))}
          </div>
          <span className="font-mono text-muted">→</span>
          <div className="flex flex-1 gap-1.5">
            {["N", "A", "F"].map((letter) => (
              <div key={letter} className="flex-1 rounded-md border border-[#9747FF] bg-surface p-2">
                <span className="grid aspect-square place-items-center rounded bg-soft text-[14px] text-accent-text">
                  {letter}
                </span>
                <i className="mt-2 block h-1.5 rounded bg-text/20" />
                <i className="mt-1 block h-1.5 w-2/3 rounded bg-text/10" />
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }
  return (
    <img
      src={`${import.meta.env.BASE_URL}${image}`}
      width="720"
      height="430"
      alt={alt}
      loading="lazy"
      className="aspect-[16/9] w-full border-b border-line object-cover object-top"
    />
  );
}
