import { readFile } from "node:fs/promises";
import { Page } from "~/components/chrome/Page";
import { meta as pageMeta } from "~/lib/seo";
import type { Route } from "./+types/changelog";

export const meta: Route.MetaFunction = () =>
  pageMeta({
    title: "Changelog · grafta",
    description: "What changed in grafta, newest first.",
    path: "/changelog/",
  });

// Runs at build time: CHANGELOG.md, at the repository's root, rendered once.
export async function loader() {
  const { marked } = await import("marked");
  const source = await readFile(new URL("../../../CHANGELOG.md", import.meta.url), "utf-8");
  const body = source
    .replace(/^# Changelog\s*/, "")
    .replace(/^## \[([^\]]+)\](\([^)]*\))?/gm, "## $1")
    .replace(/\]\((docs\/[^)]+)\)/g, "](https://github.com/n0ss4/grafta/blob/main/$1)");
  return { html: await marked.parse(body) };
}

export default function Changelog({ loaderData }: Route.ComponentProps) {
  return (
    <Page>
      <article className="grid grid-cols-4 px-4 py-20 lg:grid-cols-8 lg:px-0 lg:py-28">
        <p className="micro col-span-4 text-muted lg:col-start-2">— Changelog</p>
        <h1 className="col-span-4 mt-6 text-[40px] leading-[1.05] font-normal tracking-[-0.03em] sm:text-[64px] lg:col-span-5 lg:col-start-2">
          What changed, newest first.
        </h1>
        <div
          className="prose-grafta col-span-4 mt-10 max-w-[760px] lg:col-span-5 lg:col-start-2"
          // CHANGELOG.md from this repository, rendered at build time.
          dangerouslySetInnerHTML={{ __html: loaderData.html }}
        />
      </article>
    </Page>
  );
}
