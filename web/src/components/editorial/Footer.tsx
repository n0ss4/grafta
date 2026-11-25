import { Mark } from "./Mark";
import { footer } from "~/content/site";
import { href } from "~/lib/seo";

/** The footer: the mark and every link in one line, the licence under them. */
export function EditorialFooter({ home = false }: { home?: boolean }) {
  const to = (link: string) =>
    link.startsWith("http") ? link : link.startsWith("#") ? (home ? link : href(`/${link}`)) : href(link);
  const links = footer.flatMap((column) => column.links);
  return (
    <footer className="border-t border-line py-10">
      <div className="mx-auto grid max-w-[80rem] gap-5 px-4 lg:grid-cols-[auto_1fr] lg:items-center lg:px-8">
        <p className="flex items-center gap-2 text-[15px] font-semibold">
          <Mark size={18} />
          grafta
        </p>
        <nav aria-label="Footer" className="lg:justify-self-end">
          <ul className="flex flex-wrap gap-x-5 gap-y-2">
            {links.map((link) => (
              <li key={link.label}>
                <a href={to(link.href)} className="text-[13.5px] text-muted no-underline hover:text-text">
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
        <p className="text-[13px] text-faint lg:col-span-2">
          MIT licence. Not affiliated with Figma; Figma is a trademark of Figma, Inc.
        </p>
      </div>
    </footer>
  );
}
