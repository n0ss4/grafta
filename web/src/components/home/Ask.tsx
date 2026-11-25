import { useEffect, useState, type ReactNode } from "react";
import { href } from "~/lib/seo";
import { CodeXml, Contrast, Palette, Sun } from "../editorial/icons";
import { Band, Head, Wrap } from "./Section";

// What one can ask about a design, each request beside its answer. The answers
// are grafta's on the example file: its variables as list_variables reads them,
// its Light mode as render_node draws it, check_contrast's ratios, and a page
// built from its values.

const VARIABLES: [name: string, dark: string][] = [
  ["Background", "#0E0E12"],
  ["Surface", "#1A1A21"],
  ["Ink", "#F5F3EE"],
  ["Muted", "#A19FA9"],
  ["Track", "#2C2C35"],
  ["Accent", "#FF6B4A"],
];

const CONTRAST: [text: string, colour: string, ratio: string][] = [
  ["Golden Hour Drive", "#16161C", "15.99"],
  ["Lumen Coast", "#6A6872", "4.85"],
  ["Now playing", "#6A6872", "4.85"],
  ["1:48 and 3:52", "#6A6872", "4.85"],
];

const ASKS: { ask: string; Icon: typeof Palette; answer: ReactNode; shows: ReactNode }[] = [
  {
    ask: "What colours does this design use?",
    Icon: Palette,
    answer: (
      <>
        <b className="font-[510]">Seven</b>, each in light and dark. In dark:
      </>
    ),
    shows: (
      <ul className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
        {VARIABLES.map(([name, hex]) => (
          <li key={name} className="grid gap-1.5 rounded-[10px] p-2.5 text-[13px] ring-1 ring-line">
            <i className="h-11 rounded-md ring-1 ring-black/10 ring-inset" style={{ background: hex }} />
            {name}
            <code className="font-mono text-[11px] text-faint">{hex}</code>
          </li>
        ))}
      </ul>
    ),
  },
  {
    ask: "Make it light.",
    Icon: Sun,
    answer: (
      <>
        <b className="font-[510]">Done</b>, in the same Figma file.
      </>
    ),
    shows: (
      <div className="flex gap-4">
        {[
          ["Before", "after.webp", "The design in its dark mode"],
          ["After", "light.webp", "The same design in its light mode"],
        ].map(([label, file, alt]) => (
          <figure key={label} className="grid w-[42%] max-w-[10.625rem] gap-2">
            <img
              src={href(`showcase/player/${file}`)}
              alt={alt}
              width={480}
              height={907}
              loading="lazy"
              className="w-full rounded-[2px] ring-1 ring-line"
            />
            <figcaption className={`text-[12px] ${label === "After" ? "text-accent-text" : "text-muted"}`}>
              {label}
            </figcaption>
          </figure>
        ))}
      </div>
    ),
  },
  {
    ask: "Can every text be read?",
    Icon: Contrast,
    answer: (
      <>
        <b className="font-[510]">Yes:</b> all five pass WCAG AA, in the light mode.
      </>
    ),
    shows: (
      <ul className="max-w-[35rem] border-t border-line">
        {CONTRAST.map(([text, colour, ratio]) => (
          <li
            key={text}
            className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-4 border-b border-line py-2.5 text-[13.5px]"
          >
            <span
              className="justify-self-start rounded-[5px] bg-[#F4F1EC] px-2 py-1 text-[13px] font-semibold"
              style={{ color: colour }}
            >
              {text}
            </span>
            <span className="tabular-nums">{ratio} : 1</span>
            <span className="text-[12px] text-accent-text">Passes</span>
          </li>
        ))}
      </ul>
    ),
  },
  {
    ask: "Make this screen a web page.",
    Icon: CodeXml,
    answer: (
      <>
        <b className="font-[510]">Here it is</b>, with the design's colours and sizes.
      </>
    ),
    shows: (
      <img
        src={href("showcase/player/web.webp")}
        alt="The web page built from the design: the cover, the song, the progress bar and the controls"
        width={1100}
        height={560}
        loading="lazy"
        className="w-full max-w-[38.75rem] rounded-lg ring-1 ring-line"
      />
    ),
  },
];

const HOLD = 4600;

export function Ask() {
  const [at, setAt] = useState(0);
  const [auto, setAuto] = useState(true);

  // The requests take turns until the visitor picks one, and not at all for one who wants no motion.
  useEffect(() => {
    if (!auto || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const timer = setTimeout(() => {
      setAt((i) => (i + 1) % ASKS.length);
    }, HOLD);
    return () => {
      clearTimeout(timer);
    };
  }, [at, auto]);

  return (
    <Band id="ask" labelledBy="ask-title" className="overflow-x-clip">
      <Wrap>
        <Head
          id="ask-title"
          title="Ask it anything"
          lead="It sees the design as it is, and changes it when you ask."
          rest="Ask questions, ask for changes, or use it to make something new, in your own words."
        />
        <div className="relative mt-16 grid gap-3.5 lg:block lg:h-[35rem]">
          <div className="relative z-10 overflow-hidden rounded-[14px] bg-surface shadow-[0_30px_60px_-20px_var(--shadow)] ring-1 ring-line lg:absolute lg:top-0 lg:-left-2.5 lg:w-[27.5rem]">
            <p className="flex h-12 items-center gap-2.5 border-b border-line px-4 text-[13px] text-muted">
              <b className="font-[510] text-text">Chat</b>
              player.fig
            </p>
            <div role="group" aria-label="Requests" className="grid gap-2 p-3.5">
              {ASKS.map(({ ask, Icon }, i) => (
                <button
                  key={ask}
                  type="button"
                  aria-pressed={i === at}
                  onClick={() => {
                    setAuto(false);
                    setAt(i);
                  }}
                  className="group flex cursor-pointer items-start gap-3 rounded-[10px] p-3 text-left text-[14.5px] leading-snug text-muted ring-transparent hover:text-text aria-pressed:bg-raised aria-pressed:text-text aria-pressed:ring-1 aria-pressed:ring-line"
                >
                  <span className="grid size-6 flex-none place-items-center rounded-[7px] bg-raised ring-1 ring-line group-aria-pressed:bg-accent-text group-aria-pressed:text-bg group-aria-pressed:ring-0">
                    <Icon size={14} strokeWidth={2} />
                  </span>
                  {ask}
                </button>
              ))}
            </div>
          </div>
          <div className="overflow-hidden rounded-[14px] bg-surface ring-1 ring-line lg:absolute lg:top-12 lg:right-[-7.5rem] lg:bottom-0 lg:left-[20.625rem] lg:[mask-image:linear-gradient(to_right,#000_62%,transparent)]">
            <p className="flex h-12 items-center border-b border-line px-5 text-[13px] font-[510] lg:pl-[8.75rem]">
              Answer
            </p>
            <div aria-live="polite" className="p-5 lg:py-7 lg:pr-7 lg:pl-[8.75rem]">
              {ASKS.map(({ ask, answer, shows }, i) => (
                <div key={ask} hidden={i !== at}>
                  <p className="mb-4 text-[14.5px]">{answer}</p>
                  {shows}
                </div>
              ))}
            </div>
          </div>
        </div>
      </Wrap>
    </Band>
  );
}
