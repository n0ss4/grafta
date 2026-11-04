import { Mark } from "./Mark";
import { href } from "~/lib/seo";
import { GITHUB } from "~/content/site";
import { GitHubMark } from "./icons";
import { ThemeToggle } from "./ThemeToggle";

const links = [
  { label: "Why", to: "#why" },
  { label: "See it", to: "#client" },
  { label: "Jobs", to: "#jobs" },
  { label: "Tools", to: "tools/" },
  { label: "Install", to: "#install" },
];

/** The nav as a ruler: its links on the grid's columns, a tick under each. */
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
            className="relative hidden h-full items-center text-[14px] font-medium text-text no-underline after:absolute after:bottom-0 after:left-0 after:h-2 after:w-px after:bg-text/50 hover:text-accent-text lg:flex"
          >
            {link.label}
          </a>
        ))}
        <div className="col-span-3 flex items-center justify-end gap-5 lg:col-span-2 lg:pr-4">
          <a href={at("#install")} className="text-[14px] font-medium text-text no-underline lg:hidden">
            Install
          </a>
          <a
            href={GITHUB}
            className="inline-flex items-center gap-1.5 text-[14px] font-medium text-text no-underline hover:text-accent-text"
          >
            <GitHubMark size={14} />
            GitHub ↗
          </a>
          <span aria-hidden="true" className="h-5 w-px bg-line" />
          <ThemeToggle />
        </div>
      </nav>
    </header>
  );
}
