import { Page } from "~/components/chrome/Page";
import type { Recording } from "~/components/client/types";
import { Demo } from "~/components/demo/Demo";
import { Ask } from "~/components/home/Ask";
import { Hero } from "~/components/home/Hero";
import { Install } from "~/components/home/Install";
import { Questions } from "~/components/home/Questions";
import { Difference } from "~/components/home/Difference";
import { Trust } from "~/components/home/Trust";
import { Picker } from "~/components/install/Picker";
import { meta as pageMeta, SITE } from "~/lib/seo";
import type { Route } from "./+types/home";

export const meta: Route.MetaFunction = () => [
  ...pageMeta({
    title: "grafta · Ask your AI. Get a Figma file back.",
    description:
      "An open-source MCP server that lets your AI agent read and edit Figma files on your disk, offline, with no account: .fig, .deck, .jam and .site.",
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

// The demo's real recording is read at build time and pre-rendered without JavaScript.
export async function loader() {
  const { readFile } = await import("node:fs/promises");
  const path = await import("node:path");
  const recording = JSON.parse(
    await readFile(path.join(process.cwd(), "public", "sessions", "image.claude-code.json"), "utf8"),
  ) as Recording;
  return { recording };
}

export default function Home({ loaderData }: Route.ComponentProps) {
  return (
    <Page home>
      <Hero />
      <Demo recording={loaderData.recording} />
      <Ask />
      <Difference />
      <Trust />
      <Questions />
      <Install />
      <Picker />
    </Page>
  );
}
