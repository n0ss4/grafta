import { useEffect, useState } from "react";
import { Page } from "~/components/chrome/Page";
import { FamilyRow, FamilySide, familyId } from "~/components/reference/FamilyNav";
import { filterFamilies } from "~/components/reference/filter";
import { ToolRow } from "~/components/reference/ToolRow";
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

const toolNames = new Set(tools.families.flatMap((family) => family.tools.map((tool) => tool.name)));

/**
 * The tool reference: every tool closed, one line each, by family; open one, or
 * all, for its words and arguments. Search finds a tool by what it does. A link to
 * a tool opens it (internal: specs/105).
 */
export default function Tools({ loaderData }: Route.ComponentProps) {
  const { families, count } = loaderData;
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<Set<string>>(() => new Set());
  const shown = filterFamilies(families, query);
  const matches = shown.reduce((n, family) => n + family.tools.length, 0);
  const allOpen = open.size >= count;

  // A link to a tool opens it and brings it into view, on load and after.
  useEffect(() => {
    const follow = () => {
      const name = decodeURIComponent(window.location.hash.slice(1));
      if (!toolNames.has(name)) return;
      setOpen((before) => new Set([...before, name]));
      requestAnimationFrame(() => {
        document.getElementById(name)?.scrollIntoView({ block: "start" });
      });
    };
    const first = requestAnimationFrame(follow);
    window.addEventListener("hashchange", follow);
    return () => {
      cancelAnimationFrame(first);
      window.removeEventListener("hashchange", follow);
    };
  }, []);

  // What the search finds outside a tool's name opens, so the match shows.
  const search = (next: string) => {
    setQuery(next);
    const inside = filterFamilies(families, next).flatMap((family) =>
      family.tools.filter((found) => found.inside).map((found) => found.tool.name),
    );
    if (inside.length > 0) setOpen((before) => new Set([...before, ...inside]));
  };

  const toggle = (name: string, isOpen: boolean) => {
    setOpen((before) => {
      if (before.has(name) === isOpen) return before;
      const next = new Set(before);
      if (isOpen) next.add(name);
      else next.delete(name);
      return next;
    });
  };

  return (
    <Page>
      <div className="grid grid-cols-[minmax(0,1fr)] px-4 pt-14 pb-24 lg:grid-cols-[calc(100%/8*2)_minmax(0,1fr)] lg:px-0 lg:pt-20">
        <FamilySide families={families} />
        <div className="lg:pr-[calc(100%/8)]">
          <h1 className="text-[40px] leading-[1.05] font-semibold tracking-[-0.03em] sm:text-[48px]">
            Tool reference
          </h1>
          <p className="mt-4 max-w-[640px] text-[17px] leading-relaxed text-muted">
            The {count} tools grafta's server lists, in its own words: what your AI reads when it connects.
            Every call but <code>open_file</code>, <code>new_file</code> and <code>list_open</code> takes the{" "}
            <code>handle</code> that <code>open_file</code> returns.
          </p>
          <p className="mt-3 text-[15px] text-muted">
            From a terminal: <code>grafta tool &lt;name&gt; &apos;&lt;json&gt;&apos;</code>
          </p>
          <FamilyRow families={families} />
          <ToolSearch
            query={query}
            onQuery={search}
            matches={matches}
            total={count}
            allOpen={allOpen}
            onOpenAll={(openAll) => {
              setOpen(openAll ? new Set(toolNames) : new Set());
            }}
          />
          {shown.map((family) => (
            <section
              key={family.title}
              id={familyId(family.title)}
              aria-labelledby={`${familyId(family.title)}-title`}
              className="mb-12 scroll-mt-[9.5rem]"
            >
              <div className="mb-3 flex flex-wrap items-baseline gap-x-3">
                <h2
                  id={`${familyId(family.title)}-title`}
                  className="text-[24px] font-medium tracking-[-0.01em]"
                >
                  {family.title}
                </h2>
                <p className="text-[15px] text-muted">{family.blurb}</p>
              </div>
              <div className="overflow-hidden rounded-2xl border border-line bg-surface">
                {family.tools.map(({ tool }) => (
                  <ToolRow key={tool.name} tool={tool} open={open.has(tool.name)} onToggle={toggle} />
                ))}
              </div>
            </section>
          ))}
          {shown.length === 0 ? (
            <p className="text-muted">
              No tool, argument or description holds “{query}”. Clear the search to see all {count}.
            </p>
          ) : null}
        </div>
      </div>
    </Page>
  );
}
