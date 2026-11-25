import { Page } from "~/components/chrome/Page";
import { Ask } from "~/components/home/Ask";
import { Closing } from "~/components/home/Closing";
import { Figures } from "~/components/home/Figures";
import { Files } from "~/components/home/Files";
import { Hero } from "~/components/home/Hero";
import { Install } from "~/components/home/Install";
import { Live } from "~/components/home/Live";
import { Questions } from "~/components/home/Questions";
import { meta as pageMeta, SITE } from "~/lib/seo";
import type { Route } from "./+types/home";

export const meta: Route.MetaFunction = () => [
  ...pageMeta({
    title: "Talk to your Figma | grafta",
    description:
      "grafta connects your AI to Figma. Ask it anything about your designs: what is in a file, a change, or something new made from it, on a file or the one you have open in Figma. Free and open source.",
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

/**
 * The home page: the line and the product at work, the files it works on,
 * what it does, the panel on the open file, what one can ask, the questions
 * people ask first, and how to get it.
 */
export default function Home() {
  return (
    <Page home>
      <Hero />
      <Files />
      <Figures />
      <Live />
      <Ask />
      <Questions />
      <Install />
      <Closing />
    </Page>
  );
}
