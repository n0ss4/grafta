import { Page } from "~/components/chrome/Page";
import { Hero } from "~/components/hero/Hero";
import { Install } from "~/components/home/Install";
import { Questions } from "~/components/home/Questions";
import { Trust } from "~/components/home/Trust";
import { Picker } from "~/components/install/Picker";
import { meta as pageMeta, SITE } from "~/lib/seo";
import type { Route } from "./+types/home";

export const meta: Route.MetaFunction = () => [
  ...pageMeta({
    title: "grafta · Talk to your Figma files.",
    description:
      "Talk to any Figma file with your AI: ask what's in it, change it, or turn it into code. No Figma account, no app running, no API limits. Free and open source.",
    path: "/",
  }),
  {
    "script:ld+json": {
      "@context": "https://schema.org",
      "@type": "SoftwareApplication",
      name: "grafta",
      url: `${SITE}/`,
      description:
        "An MCP server that lets a model read and edit Figma .fig, .deck, .jam and .site files, exactly and offline.",
      applicationCategory: "DeveloperApplication",
      operatingSystem: "macOS, Linux, Windows",
      license: "https://opensource.org/licenses/MIT",
      image: `${SITE}/assets/card.png`,
      offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
      sameAs: ["https://github.com/n0ss4/grafta"],
    },
  },
];

export default function Home() {
  return (
    <Page home>
      <Hero />
      <Trust />
      <Questions />
      <Install />
      <Picker />
    </Page>
  );
}
