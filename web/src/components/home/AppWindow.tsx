import { useEffect, useState } from "react";
import { href } from "~/lib/seo";

// The product at work, as one window: the request in a chat and what grafta
// did, the design in the Figma file with the layer it changed, and that
// layer's properties before and after. The pictures are grafta's renders of a
// real file, before and after the same set_properties call; the window is
// drawn. It opens on the change made, then plays it again.

const STEPS = [
  { verb: "Opened", what: "player.fig", tool: "open_file" },
  { verb: "Read", what: "Now Playing", tool: "read_node" },
  { verb: "Changed", what: "Play button", tool: "set_properties" },
  { verb: "Drew it to check", what: "", tool: "render_node" },
] as const;

/** How far the work has gone: how many steps are done, and whether the change is made. */
interface State {
  shown: number;
  done: number;
  changed: boolean;
  said: boolean;
}
const FINISHED: State = { shown: STEPS.length, done: STEPS.length, changed: true, said: true };
const START: State = { shown: 0, done: 0, changed: false, said: false };

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export function AppWindow() {
  // Drawn finished: what a visitor without scripts, or who wants no motion, sees.
  const [state, setState] = useState<State>(FINISHED);

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    // Ended when the window leaves the page.
    const run = { live: true };
    const alive = () => run.live;
    void (async () => {
      await wait(2600);
      while (alive()) {
        setState(START);
        await wait(900);
        for (let i = 0; i < STEPS.length && alive(); i++) {
          setState((s) => ({ ...s, shown: i + 1 }));
          await wait(380);
          if (i === 2) {
            setState((s) => ({ ...s, changed: true }));
            await wait(700);
          }
          setState((s) => ({ ...s, done: i + 1 }));
          await wait(260);
        }
        setState((s) => ({ ...s, said: true }));
        await wait(4600);
      }
    })();
    return () => {
      run.live = false;
    };
  }, []);

  const { shown, done, changed, said } = state;
  const selected = shown >= 2;
  return (
    <figure className="mt-16 lg:mt-20">
      <div
        role="img"
        aria-label="A request in a chat changes the play button of a design in a Figma file, through grafta"
        className="relative -mx-1 grid h-[37.5rem] overflow-hidden rounded-[14px] bg-surface ring-1 ring-line lg:-mx-5 lg:h-[40rem] lg:grid-cols-[18.75rem_minmax(0,1fr)_17.5rem] [mask-image:linear-gradient(to_bottom,#000_84%,transparent)]"
      >
        {/* the chat */}
        <div className="hidden min-w-0 flex-col border-r border-line md:flex">
          <Bar>
            <b className="font-[510] text-text">Chat</b>
            <span>player.fig</span>
          </Bar>
          <div className="grid content-start gap-4 p-4 text-[13.5px]">
            <p className="rounded-[10px] bg-raised px-3 py-2.5 leading-snug ring-1 ring-line">
              Make the play button round and coral.
            </p>
            <ul className="grid gap-2">
              {STEPS.map((step, i) => (
                <li
                  key={step.tool}
                  className={`flex items-center gap-2 text-[12.5px] text-muted transition-[opacity,transform] duration-300 ${
                    i < shown ? "opacity-100" : "translate-y-1 opacity-0"
                  }`}
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.4"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                    className={`flex-none text-accent-text transition-opacity ${i < done ? "opacity-100" : "opacity-0"}`}
                  >
                    <path d="M5 12.5l4.5 4.5L19 7.5" />
                  </svg>
                  <span>
                    {step.verb} {step.what ? <b className="font-[510] text-text">{step.what}</b> : null}
                  </span>
                  <code className="ml-auto font-mono text-[11.5px] text-faint">{step.tool}</code>
                </li>
              ))}
            </ul>
            <p
              className={`leading-normal transition-opacity duration-500 ${said ? "opacity-100" : "opacity-0"}`}
            >
              Done. The play button is round and uses the file's <b className="font-[510]">Accent</b> colour,
              in light and dark.
            </p>
          </div>
          <p className="mx-3 mt-auto mb-3 rounded-[10px] p-3 text-[13px] text-faint ring-1 ring-line">
            Ask about your design…
          </p>
        </div>

        {/* the file */}
        <div className="relative min-w-0 bg-sunk [background-image:radial-gradient(var(--hair)_1px,transparent_1px)] [background-size:18px_18px]">
          <Bar>
            <span>player.fig</span>
            <span>/</span>
            <b className="font-[510] text-text">Now Playing</b>
            <span className="ml-auto">Dark mode</span>
          </Bar>
          <div className="grid justify-items-center pt-8">
            <div className="w-[12.5rem]">
              <p className="mb-1.5 text-[11.5px] text-muted">Now Playing</p>
              <div className="relative aspect-[360/680] overflow-hidden rounded-[2px] shadow-[0_0_0_1px_var(--hair),0_30px_60px_-20px_var(--shadow)]">
                <img
                  src={href("showcase/player/before.webp")}
                  alt=""
                  width={480}
                  height={907}
                  className="absolute inset-0 size-full"
                />
                <img
                  src={href("showcase/player/after.webp")}
                  alt=""
                  width={480}
                  height={907}
                  className="absolute inset-0 size-full transition-[clip-path] duration-700 ease-[cubic-bezier(.65,0,.35,1)]"
                  style={{ clipPath: changed ? "inset(0 0 0 0)" : "inset(0 100% 0 0)" }}
                />
                {/* the layer grafta selected and changed */}
                <span
                  className={`absolute top-[81.6%] left-[38.9%] h-[11.8%] w-[22.2%] rounded-[2px] border-[1.5px] border-accent-text transition-opacity ${
                    selected ? "opacity-100" : "opacity-0"
                  }`}
                />
                <span
                  className={`absolute top-[calc(93.4%+4px)] left-[38.9%] rounded-[3px] bg-accent-text px-1.5 text-[10px] font-semibold whitespace-nowrap text-bg transition-opacity ${
                    selected ? "opacity-100" : "opacity-0"
                  }`}
                >
                  Play button
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* the layer's properties */}
        <div className="hidden min-w-0 border-l border-line lg:block">
          <Bar>
            <b className="font-[510] text-text">Play button</b>
            <span className="ml-auto">Rectangle</span>
          </Bar>
          <div className="grid gap-[18px] p-4 text-[13px]">
            <Group title="Shape">
              <Row name="Size" value="72 × 72" />
              <Row name="Corners" value={changed ? <Was from="18" to="36" /> : "18"} />
            </Group>
            <Group title="Fill">
              <Row
                name="Colour"
                value={
                  changed ? (
                    <>
                      <s className="text-faint">Ink</s>
                      <Swatch colour="#FF6B4A" />
                      <span className="text-accent-text">Accent</span>
                    </>
                  ) : (
                    <>
                      <Swatch colour="#F5F3EE" />
                      Ink
                    </>
                  )
                }
              />
            </Group>
            <Group title="Variables">
              <Row name="Theme" value="Dark" />
              <Row
                name="Accent"
                value={
                  <>
                    <Swatch colour="#FF6B4A" />
                    #FF6B4A
                  </>
                }
              />
            </Group>
          </div>
        </div>
      </div>
      <figcaption className="mt-3 text-[13px] text-muted">
        Drawn from a real design: grafta read it, changed it and drew it, before and after.
      </figcaption>
    </figure>
  );
}

function Bar({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-12 flex-none items-center gap-2.5 border-b border-line px-4 text-[13px] text-muted">
      {children}
    </div>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <p className="mb-2 text-[11.5px] font-[510] tracking-[0.02em] text-faint">{title}</p>
      {children}
    </div>
  );
}

function Row({ name, value }: { name: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2 py-[5px]">
      <span className="text-muted">{name}</span>
      <span className="ml-auto flex items-center gap-1.5 tabular-nums">{value}</span>
    </div>
  );
}

function Was({ from, to }: { from: string; to: string }) {
  return (
    <>
      <s className="text-faint">{from}</s>
      <span className="text-accent-text">{to}</span>
    </>
  );
}

function Swatch({ colour }: { colour: string }) {
  return (
    <i className="size-3 rounded-[3px] ring-1 ring-black/15 ring-inset" style={{ background: colour }} />
  );
}
