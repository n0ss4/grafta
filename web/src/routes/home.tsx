import { useState } from "react";
import { Page } from "~/components/chrome/Page";
import { Comparison } from "~/components/client/Comparison";
import type { Recording } from "~/components/client/types";
import { Hero } from "~/components/home/Hero";
import { Install } from "~/components/home/Install";
import { Questions } from "~/components/home/Questions";
import { Difference, Jobs, Layers, Principles, Proof } from "~/components/home/Sections";
import { clients } from "~/content/clients";
import { meta as pageMeta, SITE } from "~/lib/seo";
import type { Route } from "./+types/home";

export const meta: Route.MetaFunction = () => [
  ...pageMeta({
    title: "grafta · Your agent, on your Figma files",
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

// Runs at build time only: the highlighter stays out of the browser.
export async function loader() {
  const { codeToHtml } = await import("shiki");
  const highlighted = Object.fromEntries(
    await Promise.all(
      clients.map(async (client) => [
        client.id,
        await codeToHtml(client.snippet, {
          lang: client.kind === "json" ? "json" : client.kind === "command" ? "shellscript" : "text",
          themes: { light: "github-light", dark: "github-dark" },
          defaultColor: "light",
        }),
      ]),
    ),
  ) as Record<string, string>;
  // The comparison's two recordings, read at build time: pre-rendered, no script.
  const { readFile } = await import("node:fs/promises");
  const path = await import("node:path");
  const read = async (name: string) =>
    JSON.parse(
      await readFile(path.join(process.cwd(), "public", "sessions", `${name}.json`), "utf8"),
    ) as Recording;
  // The session the hero plays on load: the first in sessions.json, in the first client it was recorded in.
  const { sessions } = await import("~/content/sessions");
  const { default: index } = await import("~/generated/session-index.json");
  const lead = sessions[0]?.id ?? "image";
  const leadClients = (index as Record<string, string[]>)[lead] ?? ["claude-code"];
  const leadClient = leadClients.includes("claude-desktop")
    ? "claude-desktop"
    : (leadClients[0] ?? "claude-code");
  const first = await read(`${lead}.${leadClient}`);
  // The comparison shows the prompt and the replies only: the calls stay out of the page's data.
  const replies = (r: Recording): Recording => ({ ...r, turns: r.turns.filter((t) => t.kind !== "tools") });
  const without = replies(await read("theme-without.claude-code"));
  const withGrafta = replies(await read(`theme.${without.client}`));
  return { highlighted, first, without, withGrafta };
}

export default function Home({ loaderData }: Route.ComponentProps) {
  const { highlighted, first, without, withGrafta } = loaderData;
  const [client, setClient] = useState("claude-code");

  return (
    <Page home>
      <Hero first={first} client={client} onClient={setClient} />
      <Comparison without={without} withGrafta={withGrafta} />
      <Layers />
      <Principles />
      <Difference />
      <Jobs />
      <Proof />
      <Install highlighted={highlighted} client={client} onClient={setClient} />
      <Questions />
    </Page>
  );
}
