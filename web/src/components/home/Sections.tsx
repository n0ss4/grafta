import { useEffect, useState } from "react";
import { cn } from "~/lib/cn";
import { useInView } from "~/lib/inView";
import { useReducedMotion } from "~/lib/motion";
import { comparison, jobs, principles, proof } from "~/content/editorial";
import { limits } from "~/content/cards";
import { products } from "~/content/products";
import { Rich } from "../Rich";
import { ArrowRight, FigmaMark } from "../editorial/icons";
import { Section } from "../editorial/Section";
import { NodeInspector } from "../live/NodeInspector";

const title = "text-[36px] leading-[1.1] font-normal tracking-[-0.03em] sm:text-[48px]";

/** 02: every node, as Figma shows it and as grafta reads it. */
export function Layers() {
  return (
    <Section id="layers" index="02" label="Every node" tone="ink">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2 lg:flex lg:items-end lg:justify-between lg:gap-10">
          <h2 id="layers-title" className={title}>
            Every node.
            <br />
            Every value.
            <br />
            <span className="text-bud">Exactly.</span>
          </h2>
          <p className="mt-6 max-w-[400px] text-[17px] leading-relaxed text-muted lg:mt-0">
            Pick a layer and see what your agent gets back: read from the file's bytes, in the plugin API's
            names, with every variable binding. What grafta cannot read, it names.
          </p>
        </div>
      </div>
      <NodeInspector />
    </Section>
  );
}

/** 03: four principles, numbered on rules. */
export function Principles() {
  return (
    <Section id="why" index="03" label="How it works">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <h2 id="why-title" className={cn(title, "col-span-4 lg:col-span-5 lg:col-start-2")}>
          What it holds to.
        </h2>
      </div>
      <div className="mt-16 grid grid-cols-4 lg:grid-cols-8">
        <ol className="col-span-4 grid grid-cols-1 gap-12 sm:grid-cols-2 lg:col-span-6 lg:col-start-2 lg:grid-cols-4 lg:gap-0">
          {principles.map((p, i) => (
            <li key={p.title} className="lift lg:pr-8" style={{ "--i": i } as React.CSSProperties}>
              <span className="draw-rule block h-px bg-text" style={{ "--i": i } as React.CSSProperties} />
              <p className="micro mt-4">0{i}</p>
              <h3 className="mt-6 text-[40px] leading-none font-normal tracking-[-0.03em]">{p.title}</h3>
              <p className="mt-5 text-[15px] leading-relaxed text-muted">{p.text}</p>
            </li>
          ))}
        </ol>
      </div>
    </Section>
  );
}

/** 04: not Figma's MCP server: the other half. */
export function Difference() {
  return (
    <Section id="difference" index="04" label="Not Figma's MCP server">
      <div className="mt-6 grid grid-cols-4 gap-y-6 lg:grid-cols-8">
        <h2 id="difference-title" className={cn(title, "col-span-4 lg:col-span-3 lg:col-start-2")}>
          The other half.
        </h2>
        <p className="col-span-4 text-[17px] leading-relaxed text-muted lg:col-span-3 lg:col-start-5">
          Figma's server works live in Figma, through your account. grafta works on the files themselves. They
          do different jobs.
        </p>
      </div>
      <div className="mt-14 grid grid-cols-4 lg:grid-cols-8">
        <table className="col-span-4 w-full border-collapse text-left lg:col-span-6 lg:col-start-2">
          <thead>
            <tr className="micro text-muted">
              <th scope="col" className="pb-3 font-medium">
                <span className="sr-only">Aspect</span>
              </th>
              <th scope="col" className="pb-3 font-medium text-accent-text">
                grafta
              </th>
              <th scope="col" className="pb-3 font-medium">
                <span className="inline-flex items-center gap-1.5">
                  <FigmaMark size={12} />
                  Figma's MCP server
                </span>
              </th>
            </tr>
          </thead>
          <tbody>
            {comparison.map((row) => (
              <tr key={row.aspect} className="border-t border-line align-baseline">
                <th scope="row" className="w-1/6 py-5 pr-4 text-[15px] font-medium text-muted">
                  {row.aspect}
                </th>
                <td className="w-2/6 py-5 pr-6 text-[17px] leading-snug">{row.grafta}</td>
                <td className="py-5 text-[15px] leading-relaxed text-muted">{row.figma}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="micro col-span-4 mt-6 text-muted lg:col-start-2">
          Figma's documentation, 4 November 2026
        </p>
      </div>
      <div className="mt-24 grid grid-cols-4 lg:grid-cols-8">
        <div id="limits" className="col-span-4 lg:col-span-6 lg:col-start-2">
          <h3 className="micro text-muted">What it does not do</h3>
          <ul className="mt-6 grid gap-x-10 sm:grid-cols-2">
            {limits.map((limit) => (
              <li key={limit.title} className="border-t border-line py-5">
                <h4 className="text-[17px] font-medium">{limit.title}</h4>
                <p className="mt-1.5 text-[15px] leading-relaxed text-muted">
                  <Rich text={limit.text} />
                </p>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}

/** A job with a session plays it in the hero's client; one without is plain text. */
function JobRow({ session, children }: { session: string | null; children: React.ReactNode }) {
  if (!session) return <>{children}</>;
  return (
    <a
      href="#client"
      className="block text-inherit no-underline"
      onClick={() => {
        window.dispatchEvent(new CustomEvent("grafta:session", { detail: session }));
      }}
    >
      {children}
    </a>
  );
}

/** 05: what you can ask, a numbered index; the products it works in. */
export function Jobs() {
  return (
    <Section id="jobs" index="05" label="What you can ask" tone="ink">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <h2 id="jobs-title" className={cn(title, "col-span-4 lg:col-span-5 lg:col-start-2")}>
          Say what you want done.
        </h2>
      </div>
      <ol className="mt-14 grid grid-cols-4 lg:grid-cols-8">
        {jobs.map((job, i) => (
          <li key={job.title} className="col-span-4 lg:col-span-6 lg:col-start-2">
            <JobRow session={job.session}>
              <div className="group grid grid-cols-[3rem_1fr] items-baseline gap-x-4 gap-y-2 border-t border-line py-6 lg:grid-cols-[4rem_minmax(0,1.1fr)_minmax(0,1.4fr)_3.5rem]">
                <span className="micro text-muted">00{i + 1}</span>
                <h3 className="text-[22px] leading-tight font-normal tracking-[-0.02em] transition-colors group-hover:text-bud">
                  {job.title}
                </h3>
                <p className="col-start-2 text-[15px] leading-relaxed text-muted lg:col-start-3">
                  “{job.prompt}”<span className="micro mt-2 block">{job.tools.join(" · ")}</span>
                </p>
                {job.session && (
                  <span className="micro hidden items-center gap-1.5 text-paper group-hover:text-bud lg:inline-flex">
                    Play
                    <ArrowRight size={14} className="transition-transform group-hover:translate-x-1" />
                  </span>
                )}
              </div>
            </JobRow>
          </li>
        ))}
      </ol>
      <div className="mt-24 grid grid-cols-4 lg:grid-cols-8">
        <div id="products" className="col-span-4 lg:col-span-6 lg:col-start-2">
          <h3 className="micro text-muted">In every Figma product, the same calls</h3>
          <ul className="mt-6 grid gap-x-10 sm:grid-cols-2">
            {products.map((product) => (
              <li key={product.name} className="border-t border-line py-5">
                <h4 className="flex items-baseline justify-between gap-4 text-[22px] font-normal tracking-[-0.02em]">
                  {product.name}
                  <span className="micro text-accent-text">{product.extension}</span>
                </h4>
                <p className="mt-2 text-[15px] leading-relaxed text-muted">{product.summary}</p>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}

function CountUp({ to, shown, run }: { to: number; shown: string; run: boolean }) {
  const reduce = useReducedMotion();
  const [value, setValue] = useState<string | null>(null);
  useEffect(() => {
    if (!run || reduce) return;
    const start = performance.now();
    let frame = 0;
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / 900);
      const eased = 1 - (1 - t) ** 3;
      const n = to * eased;
      setValue(t < 1 ? (Number.isInteger(to) ? Math.round(n).toLocaleString("en") : n.toFixed(1)) : null);
      if (t < 1) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [run, reduce, to]);
  return <>{value ?? shown}</>;
}

/** 06: built with grafta, the figures on rules. */
export function Proof() {
  const { ref, seen } = useInView<HTMLOListElement>(0.4);
  return (
    <Section id="proof" index="06" label="Measured">
      <div className="mt-6 grid grid-cols-4 lg:grid-cols-8">
        <h2 id="proof-title" className={cn(title, "col-span-4 lg:col-span-5 lg:col-start-2")}>
          Measured on public Figma files.
        </h2>
      </div>
      <div className="mt-16 grid grid-cols-4 lg:grid-cols-8">
        <ol
          ref={ref}
          className="col-span-4 grid grid-cols-1 gap-10 sm:grid-cols-2 lg:col-span-6 lg:col-start-2 lg:grid-cols-4 lg:gap-0"
        >
          {proof.map((fig, i) => (
            <li key={fig.label} className="lg:pr-8">
              <span className="draw-rule block h-px bg-text" style={{ "--i": i } as React.CSSProperties} />
              <p className="mt-5 text-[40px] leading-none font-normal tracking-[-0.04em] tabular-nums xl:text-[48px]">
                <CountUp to={fig.value} shown={fig.shown} run={seen} />
              </p>
              <p className="mt-4 text-[15px] leading-relaxed text-muted">{fig.label}</p>
            </li>
          ))}
        </ol>
      </div>
      <div className="mt-10 grid grid-cols-4 lg:grid-cols-8">
        <p className="micro col-span-4 text-muted lg:col-span-6 lg:col-start-2">
          One Mac with Apple silicon, the release build of 0.1.0, 4 November 2026. Your machine will differ.
        </p>
      </div>
    </Section>
  );
}
