import { useEffect, useMemo, useRef, useState } from "react";
import type { Recording } from "../client/types";
import { Design } from "../client/Design";
import { Cursor } from "../figma";
import { useReducedMotion } from "~/lib/motion";
import { sessions } from "~/content/sessions";
import { cn } from "~/lib/cn";

export const demoSteps = [
  { id: "ask", title: "You ask", text: "Turn this poster into a Figma file I can edit.", beat: null },
  {
    id: "open",
    title: "It starts a real .fig",
    text: "A new local file, with a 1080 × 1350 frame for the poster.",
    beat: "read",
  },
  {
    id: "change",
    title: "It rebuilds the layout",
    text: "The background, sun, hill and information band become frames, shapes and vectors.",
    beat: "layout",
  },
  {
    id: "text",
    title: "It sets real text",
    text: "Every headline and label stays editable; the button uses auto-layout.",
    beat: "text",
  },
  {
    id: "check",
    title: "It checks its work",
    text: "grafta renders the .fig, compares it with the picture and adjusts the result.",
    beat: "check",
  },
  {
    id: "save",
    title: "You get the file",
    text: "poster-editable.fig: pages, named layers, vectors and text that open in Figma.",
    beat: "save",
  },
] as const;

export function validateDemo(recording: Recording): string[] {
  const beats = new Set(recording.turns.filter((turn) => turn.kind === "tools").map((turn) => turn.beat));
  return demoSteps.flatMap((step) => (step.beat && !beats.has(step.beat) ? [step.beat] : []));
}

function turnFor(recording: Recording, beat: string | null): number {
  if (!beat) return 0;
  let found = 0;
  recording.turns.forEach((turn, index) => {
    if (turn.kind === "tools" && turn.beat === beat) found = index;
  });
  return found;
}

function Stage({ recording, active }: { recording: Recording; active: number }) {
  const spec = sessions.find((item) => item.id === "image");
  if (!spec) return null;
  const step = demoSteps[active] ?? demoSteps[0];
  const at = turnFor(recording, step.beat);
  return (
    <div className="relative min-h-[460px] overflow-hidden rounded-2xl border border-line bg-bg p-5 sm:p-8">
      <Cursor
        className={cn(
          "transition-all duration-500",
          active < 2 ? "top-24 left-[68%]" : active < 4 ? "top-[48%] left-[52%]" : "top-[72%] left-[74%]",
        )}
      />
      <div className="mt-10 flex min-h-[340px] items-center justify-center overflow-hidden">
        {active === 0 ? (
          <div className="max-w-[420px] rounded-2xl bg-ink px-5 py-4 text-[15px] leading-relaxed text-paper shadow-lg">
            “{spec.prompt}”
          </div>
        ) : (
          <div className="origin-center scale-[0.72] sm:scale-[0.82] lg:scale-[0.76] xl:scale-[0.86]">
            <Design spec={spec} recording={recording} at={at} />
          </div>
        )}
      </div>
      <div className="absolute right-5 bottom-5 left-5 flex items-center justify-between gap-4">
        <span className="font-mono text-[11px] text-muted">
          {active + 1} / {demoSteps.length}
        </span>
        <span className="h-1 flex-1 overflow-hidden rounded bg-line">
          <i
            className="block h-full bg-text transition-[width]"
            style={{ width: `${((active + 1) / demoSteps.length) * 100}%` }}
          />
        </span>
      </div>
    </div>
  );
}

/** One real session, visitor-paced: sticky on desktop, cards on phone/reduced motion. */
export function Demo({ recording }: { recording: Recording }) {
  const reduce = useReducedMotion();
  const [active, setActive] = useState(0);
  const refs = useRef<(HTMLLIElement | null)[]>([]);
  const missing = useMemo(() => validateDemo(recording), [recording]);

  useEffect(() => {
    if (reduce || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) setActive(Number((entry.target as HTMLElement).dataset.step));
        });
      },
      { rootMargin: "-40% 0px -40% 0px", threshold: 0 },
    );
    refs.current.forEach((node) => {
      if (node) observer.observe(node);
    });
    return () => {
      observer.disconnect();
    };
  }, [reduce]);

  if (missing.length) throw new Error(`The guided demo is missing recorded beats: ${missing.join(", ")}`);
  const cards = reduce ? true : undefined;
  const move = (event: React.KeyboardEvent, from: number) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const next = Math.max(0, Math.min(demoSteps.length - 1, from + (event.key === "ArrowDown" ? 1 : -1)));
    setActive(next);
    refs.current[next]?.scrollIntoView({ block: "center" });
    refs.current[next]?.querySelector("button")?.focus();
  };
  return (
    <section id="how" aria-labelledby="how-title" className="bg-sunk px-4 py-24 lg:px-0 lg:py-32">
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <h2 id="how-title" className="text-[36px] font-normal tracking-[-0.03em] sm:text-[48px]">
            How it works
          </h2>
          <p className="mt-2 text-[17px] text-muted">
            One real request, on a local copy of your file: in Figma, File → Save local copy…
          </p>
          <div className="mt-14 lg:grid lg:grid-cols-[340px_minmax(0,680px)] lg:gap-14">
            <ol className="space-y-4 lg:space-y-0">
              {demoSteps.map((step, index) => (
                <li
                  key={step.id}
                  ref={(node) => {
                    refs.current[index] = node;
                  }}
                  data-step={index}
                  data-demo-step
                  className={cn(
                    "min-h-0 border-t border-line py-5 lg:flex lg:min-h-[58vh] lg:flex-col lg:justify-center",
                    !cards && active === index && "text-accent-text",
                  )}
                >
                  <button
                    type="button"
                    onClick={() => {
                      setActive(index);
                    }}
                    onKeyDown={(event) => {
                      move(event, index);
                    }}
                    className="w-full cursor-pointer text-left"
                    aria-current={active === index ? "step" : undefined}
                  >
                    <span className="block font-mono text-[11px] text-muted">0{index + 1}</span>
                    <span className="mt-2 block text-[20px] font-medium">{step.title}</span>
                    <span className="mt-2 block text-[15px] leading-relaxed text-muted">{step.text}</span>
                  </button>
                  <div className={cn("mt-5", cards ? "lg:block" : "lg:hidden")}>
                    <Stage recording={recording} active={index} />
                  </div>
                </li>
              ))}
            </ol>
            <div className={cards ? "hidden" : "hidden lg:block"}>
              <div className="sticky top-28">
                <Stage recording={recording} active={active} />
              </div>
            </div>
          </div>
          <a
            href={`${import.meta.env.BASE_URL}sessions/image.claude-code.json`}
            className="mt-8 inline-block text-[14px] font-medium underline underline-offset-4"
          >
            Real session ↗
          </a>
        </div>
      </div>
    </section>
  );
}
