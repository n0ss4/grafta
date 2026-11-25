import { Mark } from "./Mark";
import { href } from "~/lib/seo";
import { GITHUB } from "~/content/site";
import { ThemeToggle } from "./ThemeToggle";

const links = [
  { label: "What it does", to: "#what" },
  { label: "Examples", to: "#ask" },
  { label: "Questions", to: "#faq" },
  { label: "Open a file", to: "open/" },
  { label: "Tools", to: "tools/" },
];

const pill =
  "inline-flex h-8 items-center rounded-full bg-button px-3.5 text-[14px] font-semibold whitespace-nowrap text-on-button no-underline hover:opacity-90";

/** The nav: the mark, the page's sections, and the ways out. */
export function RulerNav({ home = false }: { home?: boolean }) {
  const at = (to: string) => (to.startsWith("#") ? (home ? to : href(`/${to}`)) : href(to));
  return (
    <header className="sticky top-0 z-40 border-b border-line bg-bg/85 backdrop-blur-md">
      <nav aria-label="Site" className="mx-auto flex h-16 max-w-[80rem] items-center gap-6 px-4 lg:px-8">
        <a
          href={href("")}
          className="flex items-center gap-2.5 text-[18px] font-semibold tracking-[-0.02em] text-text no-underline"
        >
          <Mark size={22} />
          grafta
        </a>
        <div className="ml-auto flex items-center gap-6 text-[14px]">
          {links.map((link) => (
            <a
              key={link.label}
              href={at(link.to)}
              className="hidden text-muted no-underline hover:text-text lg:inline"
            >
              {link.label}
            </a>
          ))}
          <span aria-hidden="true" className="hidden h-4 w-px bg-line lg:block" />
          <a href={GITHUB} className="hidden text-muted no-underline hover:text-text sm:inline">
            GitHub
          </a>
          <ThemeToggle />
          <a href={at("#get")} className={pill}>
            Get grafta
          </a>
        </div>
      </nav>
    </header>
  );
}
