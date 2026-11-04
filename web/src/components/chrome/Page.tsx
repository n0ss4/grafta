import type { ReactNode } from "react";
import { EditorialFooter } from "../editorial/Footer";
import { RulerNav } from "../editorial/RulerNav";

/** Every page: the ruler nav, the page, the footer, on the visible grid. */
export function Page({ home = false, children }: { home?: boolean; children: ReactNode }) {
  return (
    <>
      <RulerNav home={home} />
      <main id="main">{children}</main>
      <EditorialFooter />
    </>
  );
}
