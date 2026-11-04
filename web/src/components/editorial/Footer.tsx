import { Mark } from "./Mark";
import { footer } from "~/content/site";
import { href } from "~/lib/seo";

/** The footer: the columns on the grid, the brand's line, the licence. */
export function EditorialFooter() {
  return (
    <footer className="grid-lines border-t border-line px-4 py-16 lg:px-0">
      <div className="grid grid-cols-4 gap-y-10 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-2 lg:col-start-2">
          <p className="flex items-center gap-2 text-[15px] font-semibold">
            <Mark size={20} />
            grafta
          </p>
          <p className="mt-4 max-w-[300px] text-[15px] leading-relaxed text-muted">
            Figma's file format for your agent: read and write Figma files, exactly and offline.
          </p>
        </div>
        {footer.map((column) => (
          <nav key={column.title} aria-label={column.title} className="col-span-2 lg:col-span-1">
            <p className="micro text-muted">{column.title}</p>
            <ul className="mt-4 space-y-2.5">
              {column.links.map((link) => (
                <li key={link.label}>
                  <a
                    href={
                      link.href.startsWith("http") || link.href.startsWith("#") ? link.href : href(link.href)
                    }
                    className="text-[15px] font-medium text-text no-underline hover:text-accent-text"
                  >
                    {link.label}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
        <p className="micro col-span-4 text-muted lg:col-span-6 lg:col-start-2">
          MIT licence · Not affiliated with Figma. Figma is a trademark of Figma, Inc.
        </p>
        <p className="micro col-span-4 text-muted lg:col-span-6 lg:col-start-2">
          Not affiliated with Anthropic. Claude is a trademark of Anthropic. The client in the first screen is
          redrawn from the measures of MCP Apps for Claude (Figma Community, CC BY 4.0).
        </p>
      </div>
    </footer>
  );
}
