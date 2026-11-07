import { Page } from "~/components/chrome/Page";
import type { Recording } from "~/components/client/types";
import { Demo } from "~/components/demo/Demo";
import { Ask } from "~/components/home/Ask";
import { Hero } from "~/components/hero/Hero";
import { Install } from "~/components/home/Install";
import { Questions } from "~/components/home/Questions";
import { Difference } from "~/components/home/Difference";
import { Trust } from "~/components/home/Trust";
import { Picker } from "~/components/install/Picker";
import { meta as pageMeta, SITE } from "~/lib/seo";
import { talkFrom, type Layer } from "~/content/talk";
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

// The real recordings are read at build time and pre-rendered without JavaScript.
export async function loader() {
  const { readFile } = await import("node:fs/promises");
  const path = await import("node:path");
  const { marked } = await import("marked");
  const read = async (...parts: string[]) =>
    JSON.parse(await readFile(path.join(process.cwd(), "public", ...parts), "utf8")) as unknown;
  const recording = (await read("sessions", "image.claude-code.json")) as Recording;
  // The hero's conversation, recorded, reduced to what it shows (content/talk.ts).
  const talk = await talkFrom(
    (await read("sessions", "talk.claude-code.json")) as Recording,
    (await read("showcase", "talk", "layers.json")) as Layer,
    (markdown) => marked.parse(markdown),
  );
  return { recording, talk };
}

export default function Home({ loaderData }: Route.ComponentProps) {
  return (
    <Page home>
      <Hero talk={loaderData.talk} />
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
