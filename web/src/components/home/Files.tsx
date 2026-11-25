import { products } from "~/content/products";
import { ProductMark } from "./ProductMark";
import { Band, Wrap } from "./Section";

/** The files grafta reads and writes, as README.md lists them: one line under the window. */
export function Files() {
  return (
    <Band id="files" labelledBy="files-title" padding="py-12 lg:py-14">
      <Wrap>
        <h2 id="files-title" className="text-center text-[13px] font-normal text-muted">
          Every kind of Figma file
        </h2>
        <ul className="mt-6 grid grid-cols-2 gap-6 sm:grid-cols-4">
          {products.map((product) => (
            <li key={product.extension} className="flex items-center justify-center gap-3">
              <ProductMark extension={product.extension} size={32} />
              <span>
                <span className="block text-[17px] font-[510]">{product.extension}</span>
                <span className="block text-[13px] text-muted">{product.name}</span>
              </span>
            </li>
          ))}
        </ul>
      </Wrap>
    </Band>
  );
}
