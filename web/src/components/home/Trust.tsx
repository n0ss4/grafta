import { products } from "~/content/products";
import { ComponentMark, FrameName, Measure } from "../figma";

const reasons = [
  ["No account", "There is nothing to sign in to."],
  ["Offline", "It makes no network call of its own."],
  ["Keeps your file", "Every change goes to a new file. Only Figma can say it opens."],
  ["Open source", "The server and file parser are MIT licensed."],
] as const;

export function Trust() {
  return (
    <section
      id="trust"
      aria-labelledby="trust-title"
      className="tone-ink grid-lines px-4 py-24 lg:px-0 lg:py-32"
    >
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <FrameName>Your files stay yours</FrameName>
          <h2 id="trust-title" className="mt-6 text-[36px] font-normal tracking-[-0.03em] sm:text-[48px]">
            Your files stay yours.
          </h2>
          <div className="mt-14 grid gap-8 sm:grid-cols-2 lg:grid-cols-4">
            {reasons.map(([title, text], index) => (
              <article key={title} className="border-t border-line pt-5">
                <Measure>0{index + 1}</Measure>
                <h3 className="mt-5 text-[22px] font-normal">{title}</h3>
                <p className="mt-2 text-[14px] leading-relaxed text-muted">{text}</p>
              </article>
            ))}
          </div>
          <ul className="mt-16 grid grid-cols-2 gap-px overflow-hidden rounded-2xl bg-line sm:grid-cols-4">
            {products.map((product) => (
              <li key={product.extension} className="bg-ink p-5">
                <ComponentMark className="text-bud" />
                <p className="mt-4 text-[24px] font-medium text-bud">{product.extension}</p>
                <p className="text-[13px] text-muted">{product.name}</p>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}
