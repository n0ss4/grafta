import { useState } from "react";
import { Page } from "~/components/chrome/Page";
import { filterFamilies } from "~/components/reference/filter";
import { ToolCard } from "~/components/reference/ToolCard";
import { ToolSearch } from "~/components/reference/ToolSearch";
import type { Family } from "~/components/reference/types";
import tools from "~/generated/tools.json";
import { meta as pageMeta } from "~/lib/seo";
import type { Route } from "./+types/tools";

export const meta: Route.MetaFunction = () =>
  pageMeta({
    title: "Tool reference · grafta",
    description: `Every one of grafta's ${tools.count} MCP tools, with its arguments, as the server lists them.`,
    path: "/tools/",
  });

export function loader() {
  return { families: tools.families as Family[], count: tools.count };
}

const slug = (title: string) => title.toLowerCase().replace(/\s+/g, "-");

export default function Tools({ loaderData }: Route.ComponentProps) {
  const { families, count } = loaderData;
  const [query, setQuery] = useState("");
  const shown = filterFamilies(families, query);
  const matches = shown.reduce((n, family) => n + family.tools.length, 0);

  return (
    <Page>
      <div className="grid-lines grid grid-cols-[minmax(0,1fr)] gap-10 px-4 py-20 lg:grid-cols-[calc(100%/8*2)_minmax(0,1fr)] lg:gap-0 lg:px-0 lg:py-28">
        <aside
          aria-label="Tools"
          className="hidden lg:sticky lg:top-24 lg:block lg:max-h-[calc(100vh-120px)] lg:self-start lg:overflow-y-auto lg:pl-4"
        >
          <ul className="gap-6 text-[14px] sm:columns-3 lg:columns-1">
            {families.map((family) => (
              <li key={family.title} className="mb-4 break-inside-avoid">
                <a
                  href={`#${slug(family.title)}`}
                  className="mb-1 block font-mono text-[12px] font-semibold tracking-[0.06em] text-muted uppercase no-underline"
                >
                  {family.title}
                </a>
                <ul>
                  {family.tools.map((tool) => (
                    <li key={tool.name}>
                      <a
                        href={`#${tool.name}`}
                        className="font-mono text-[14px] leading-loose text-text no-underline hover:text-accent-text"
                      >
                        {tool.name}
                      </a>
                    </li>
                  ))}
                </ul>
              </li>
            ))}
          </ul>
        </aside>
        <div className="lg:pr-[calc(100%/6)]">
          <p className="micro text-muted">— Tool reference</p>
          <h1 className="mt-6 text-[40px] leading-[1.05] font-normal tracking-[-0.03em] sm:text-[64px]">
            {count} tools, as the server lists them.
          </h1>
          <p className="mt-6 mb-10 max-w-[680px] text-[17px] text-muted">
            The {count} tools grafta's MCP server lists, in its own words, with their arguments: what a model
            reads when it connects. Every call but <code>open_file</code>, <code>new_file</code> and{" "}
            <code>list_open</code> takes the <code>handle</code> <code>open_file</code> returns. From a
            terminal, <code>grafta tool &lt;name&gt; '&lt;json&gt;'</code> calls one.
          </p>
          <ToolSearch query={query} onQuery={setQuery} matches={matches} total={count} />
          {shown.map((family) => (
            <section key={family.title} aria-labelledby={slug(family.title)} className="mb-12">
              <h2
                id={slug(family.title)}
                className="border-t border-line pt-6 text-[28px] font-normal tracking-[-0.02em]"
              >
                {family.title}
              </h2>
              <p className="mt-1 mb-4 text-muted">{family.blurb}</p>
              <div className="space-y-4">
                {family.tools.map((tool) => (
                  <ToolCard key={tool.name} tool={tool} />
                ))}
              </div>
            </section>
          ))}
          {shown.length === 0 && <p className="text-muted">No tool or argument holds “{query}”.</p>}
        </div>
      </div>
    </Page>
  );
}
