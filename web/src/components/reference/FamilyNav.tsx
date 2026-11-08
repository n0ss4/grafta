import { useEffect, useState } from "react";
import type { Family } from "./types";

export const familyId = (title: string) => title.toLowerCase().replace(/\s+/g, "-");

/** The family whose section is nearest under the sticky bars, as the page scrolls. */
function useFamilyInView(ids: string[]): string | undefined {
  const [current, setCurrent] = useState<string | undefined>(ids[0]);
  useEffect(() => {
    const sections = ids.flatMap((id) => {
      const node = document.getElementById(id);
      return node ? [node] : [];
    });
    const pick = () => {
      const line = 170; // below the nav and the search
      let seen = sections[0]?.id;
      for (const section of sections) if (section.getBoundingClientRect().top <= line) seen = section.id;
      setCurrent(seen);
    };
    pick();
    window.addEventListener("scroll", pick, { passive: true });
    return () => {
      window.removeEventListener("scroll", pick);
    };
  }, [ids]);
  return current;
}

/** On a desktop, the families and their tools, sticky, the family in view marked. */
export function FamilySide({ families }: { families: Family[] }) {
  const current = useFamilyInView(families.map((family) => familyId(family.title)));
  return (
    <nav
      aria-label="Families of tools"
      className="hidden lg:sticky lg:top-24 lg:block lg:max-h-[calc(100vh-120px)] lg:self-start lg:overflow-y-auto lg:pr-6 lg:pl-4"
    >
      <ul className="text-[14px]">
        {families.map((family) => {
          const id = familyId(family.title);
          const here = id === current;
          return (
            <li key={family.title} className="mb-3">
              <a
                href={`#${id}`}
                aria-current={here ? "location" : undefined}
                className={`block border-l-2 py-0.5 pl-3 font-medium no-underline ${
                  here ? "border-accent text-text" : "border-transparent text-muted hover:text-text"
                }`}
              >
                {family.title}
              </a>
              {here ? (
                <ul className="mt-1 mb-2 pl-3.5">
                  {family.tools.map((tool) => (
                    <li key={tool.name}>
                      <a
                        href={`#${tool.name}`}
                        className="block py-1 font-mono text-[13px] leading-5 text-muted no-underline hover:text-accent-text"
                      >
                        {tool.name}
                      </a>
                    </li>
                  ))}
                </ul>
              ) : null}
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

/** On a phone, the families as a row of links under the intro. */
export function FamilyRow({ families }: { families: Family[] }) {
  return (
    <nav aria-label="Families of tools" className="mt-6 lg:hidden">
      <ul className="flex flex-wrap gap-2">
        {families.map((family) => (
          <li key={family.title}>
            <a
              href={`#${familyId(family.title)}`}
              className="block rounded-full border border-line bg-surface px-3 py-1.5 text-[14px] text-text no-underline"
            >
              {family.title} <span className="text-muted">{family.tools.length}</span>
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}
