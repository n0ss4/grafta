import { href } from "~/lib/seo";
import { Band, Wrap } from "./Section";

/** The page's last word: the line again, large and centred, and the two ways on. */
export function Closing() {
  return (
    <Band id="closing" labelledBy="closing-title" padding="py-[clamp(96px,12vw,160px)]">
      <Wrap className="text-center">
        <h2
          id="closing-title"
          className="mx-auto max-w-[14ch] text-[length:clamp(40px,6vw,76px)] leading-[1] font-semibold tracking-[-0.04em] text-balance"
        >
          Talk to your Figma.
        </h2>
        <p className="mx-auto mt-6 max-w-[34rem] text-[length:clamp(17px,1.5vw,19px)] leading-[1.55] text-muted">
          Free and open source. In the AI app you already use.
        </p>
        <div className="mt-9 flex flex-wrap justify-center gap-3">
          <a
            href="#get"
            className="inline-flex h-12 items-center rounded-[10px] bg-button px-7 text-[16px] font-semibold text-on-button no-underline hover:opacity-90"
          >
            Get grafta
          </a>
          <a
            href={href("open/")}
            className="inline-flex h-12 items-center rounded-[10px] px-6 text-[16px] font-medium text-text no-underline ring-1 ring-line hover:ring-text"
          >
            Open a file in your browser
          </a>
        </div>
      </Wrap>
    </Band>
  );
}
