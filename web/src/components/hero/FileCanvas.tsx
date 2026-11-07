import type { Talk } from "~/content/talk";
import { href } from "~/lib/seo";
import { Layers } from "./Layers";

/**
 * The file, as the AI app's grafta sees it: its layers beside its picture.
 * Every request's picture is in the page, the active one shown, so switching
 * loads nothing and moves nothing.
 */
export function FileCanvas({ talk, active }: { talk: Talk; active: number }) {
  const request = talk.requests[active];
  const mode = request?.id === "light" ? "Light" : "Dark";
  return (
    <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-line bg-surface">
      <div className="flex items-center justify-between gap-3 border-b border-line px-4 py-2.5 text-[13px]">
        <span className="font-medium text-text">{talk.file}</span>
        <span className="text-muted">Theme: {mode} mode</span>
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-1 sm:grid-cols-[180px_minmax(0,1fr)]">
        <nav aria-label="Layers of order.fig" className="hidden border-r border-line py-2 sm:block">
          <ul>
            <Layers layer={talk.layers} marked={request?.id === "light"} />
          </ul>
        </nav>
        <figure className="relative m-0 flex min-h-0 flex-col items-center justify-center bg-sunk p-3 sm:p-6">
          <div className="relative aspect-[720/730] w-full max-w-[150px] sm:max-w-[300px]">
            {talk.requests.map((each, index) => (
              <img
                key={each.id}
                src={href(each.picture.src)}
                width={each.picture.width}
                height={each.picture.height}
                alt={index === active ? each.picture.alt : ""}
                aria-hidden={index !== active}
                fetchPriority={index === 0 ? "high" : "low"}
                loading={index === 0 ? "eager" : "lazy"}
                decoding="async"
                className="talk-fade absolute inset-0 h-full w-full rounded-lg shadow-[0_1px_2px_rgb(0_0_0/0.12),0_8px_24px_rgb(0_0_0/0.10)]"
                data-visible={index === active}
              />
            ))}
          </div>
          <figcaption className="mt-3 text-center text-[12px] text-muted">{request?.picture.by}</figcaption>
        </figure>
      </div>
    </div>
  );
}
