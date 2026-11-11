import { Page } from "~/components/chrome/Page";
import { Open } from "~/components/open/Open";
import { href, meta as pageMeta, SITE } from "~/lib/seo";
import type { Route } from "./+types/open";

export const meta: Route.MetaFunction = () => [
  ...pageMeta({
    title: "Open a Figma file online, without Figma · grafta",
    description:
      "Drop a .fig, .deck, .jam or .site file and see its pages and screens, download them as PNG, and take its variables as W3C design tokens. It runs in your browser: nothing is uploaded, no Figma account.",
    path: "/open/",
  }),
  {
    "script:ld+json": {
      "@context": "https://schema.org",
      "@type": "WebApplication",
      name: "Open a Figma file, by grafta",
      url: `${SITE}/open/`,
      applicationCategory: "DesignApplication",
      operatingSystem: "Any, in a web browser",
      offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
    },
  },
];

/**
 * A Figma file opened in the page: grafta's reader, compiled to WebAssembly,
 * reads it in a worker and draws its screens (internal: specs/109).
 */
export default function OpenPage() {
  return (
    <Page>
      <section aria-labelledby="open-title" className="px-4 pt-14 pb-24 lg:px-8 lg:pt-20">
        <div className="mx-auto max-w-[76rem]">
          <h1
            id="open-title"
            className="text-[40px] leading-[1.04] font-normal tracking-[-0.04em] sm:text-[56px]"
          >
            Open a Figma file, without Figma.
          </h1>
          <p className="mt-4 max-w-[40rem] text-[17px] leading-relaxed text-muted">
            See its pages and screens, download them as PNG, and take its variables as design tokens. grafta
            reads the file here, in your browser: nothing is uploaded, and no Figma account is needed.
          </p>
          <noscript>
            <p className="mt-6 text-[15px]">
              This page reads your file with JavaScript, which is off. Turn it on, or{" "}
              <a href={href("#get")}>install grafta</a> to read it in your AI app.
            </p>
          </noscript>
          <div className="mt-10">
            <Open />
          </div>
        </div>
      </section>
    </Page>
  );
}
