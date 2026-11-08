import { Mark } from "./Mark";
import { href } from "~/lib/seo";
import { GITHUB } from "~/content/site";
import { GitHubMark } from "./icons";
import { ThemeToggle } from "./ThemeToggle";
import { GetGraftaButton } from "../install/Picker";

const links = [
  { label: "See it work", to: "#hero" },
  { label: "Your files", to: "#trust" },
  { label: "Questions", to: "#faq" },
];

/** The nav: the mark, the page's sections, and the ways out. */
export function RulerNav({ home = false }: { home?: boolean }) {
  const at = (to: string) => (to.startsWith("#") ? (home ? to : href(`/${to}`)) : href(to));
  return (
    <header className="sticky top-0 z-40 border-b border-line bg-bg/85 backdrop-blur-md">
      <nav aria-label="Site" className="grid h-16 grid-cols-4 items-center px-4 lg:grid-cols-8 lg:px-0">
        <a
          href={href("")}
          className="flex items-center gap-2 text-[15px] font-semibold text-text no-underline lg:pl-4"
        >
          <Mark size={20} />
          grafta
        </a>
        {links.map((link) => (
          <a
            key={link.label}
            href={at(link.to)}
            className="relative hidden h-full items-center text-[14px] font-medium text-text no-underline hover:text-accent-text lg:flex"
          >
            {link.label}
          </a>
        ))}
        {/* the grid keeps a column for each of the page's sections */}
        <span aria-hidden="true" className="hidden lg:block" />
        {home ? (
          <GetGraftaButton className="relative hidden h-full items-center text-left text-[14px] font-medium text-text hover:text-accent-text lg:flex">
            Get grafta
          </GetGraftaButton>
        ) : (
          <a
            href={at("#get")}
            className="relative hidden h-full items-center text-[14px] font-medium text-text no-underline lg:flex"
          >
            Get grafta
          </a>
        )}
        <div className="col-span-3 flex items-center justify-end gap-4 whitespace-nowrap lg:col-span-2 lg:gap-5 lg:pr-4">
          {home ? (
            <GetGraftaButton className="text-[14px] font-medium text-text lg:hidden">
              Get grafta
            </GetGraftaButton>
          ) : (
            <a href={at("#get")} className="text-[14px] font-medium text-text no-underline lg:hidden">
              Get grafta
            </a>
          )}
          <a
            href={at("tools/")}
            className="text-[14px] font-medium text-text no-underline hover:text-accent-text"
          >
            Tools
          </a>
          <a
            href={GITHUB}
            aria-label="GitHub"
            className="inline-flex items-center gap-1.5 text-[14px] font-medium text-text no-underline hover:text-accent-text"
          >
            <GitHubMark size={14} />
            <span className="hidden sm:inline">GitHub</span>
          </a>
          <span aria-hidden="true" className="h-5 w-px bg-line" />
          <ThemeToggle />
        </div>
      </nav>
    </header>
  );
}
