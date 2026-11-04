import { useEffect, useLayoutEffect, useRef, useState } from "react";
import nodesJson from "~/generated/nodes.json";
import { cn } from "~/lib/cn";
import { OrderCard } from "./OrderCard";

interface Read {
  layer: string;
  node: Record<string, unknown> & { id: string; type: string; name: string; width?: number; height?: number };
}

/** read_node's answers for the order card's layers, recorded from grafta (docs/showcase/nodes.py). */
const reads = nodesJson as Read[];

/** Where each layer is in the OrderCard component, which draws the same layers. */
const target: Record<string, string> = {
  Order: '[data-node="FRAME Order"]',
  "Your order": '[data-node="TEXT title"]',
  Card: '[data-node="FRAME Card"]',
  Package: '[data-node="FRAME Icon"] svg',
  Progress: '[data-node="FRAME Progress"] > span',
  Button: '[data-node="FRAME Button"]',
  "Track order": '[data-node="TEXT label"]',
};

const depth: Record<string, number> = {
  Order: 0,
  "Your order": 1,
  Card: 1,
  Package: 2,
  Progress: 2,
  Button: 1,
  "Track order": 2,
};

/** The card drawn at the size the file has it, 360 px wide from the component's 232, or as wide as the canvas allows. */
const FULL = 360 / 232;

type Box = { left: number; top: number; width: number; height: number } | null;

/** One line of read_node's JSON, with variable bindings picked out. */
function JsonView({ value }: { value: Record<string, unknown> }) {
  const text = JSON.stringify(value, null, 2);
  return (
    <pre className="font-mono text-[12px] leading-[1.7] whitespace-pre-wrap text-paper/80">
      {text.split("\n").map((line, i) => {
        const m = /^(\s*)"([^"]+)":(.*)$/.exec(line);
        if (!m) return <div key={i}>{line}</div>;
        const [, indent, key, rest] = m;
        const bound = key === "variable";
        return (
          <div key={i} className={cn(bound && "-mx-2 rounded bg-bud/10 px-2")}>
            {indent}
            <span className={bound ? "text-bud" : "text-paper/50"}>"{key}"</span>:
            <span className={bound ? "text-bud" : undefined}>{rest}</span>
          </div>
        );
      })}
    </pre>
  );
}

/**
 * Figma's three panes, the right one replaced by what grafta answers: the
 * layers, the order card at its real size, and read_node for the selected
 * layer. The selection follows the scroll on a large screen, and a click or
 * the keyboard anywhere.
 */
export function NodeInspector() {
  const [index, setIndex] = useState(1);
  const [box, setBox] = useState<Box>(null);
  const [scale, setScale] = useState(FULL);
  const pane = useRef<HTMLDivElement>(null);
  const root = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLDivElement>(null);
  const lastScrolled = useRef(-1);

  /** The layer the scroll is on, or -1 where the scroll does not choose. */
  const scrolledTo = () => {
    const node = root.current;
    if (!node || !window.matchMedia("(min-width: 1024px)").matches) return -1;
    const rect = node.getBoundingClientRect();
    const travel = rect.height - window.innerHeight;
    if (travel <= 0) return -1;
    const p = Math.min(0.999, Math.max(0, -rect.top / travel));
    return Math.floor(p * reads.length);
  };
  /** A layer picked by hand stays picked until the scroll reaches another. */
  const pick = (i: number) => {
    lastScrolled.current = scrolledTo();
    setIndex(i);
  };

  // As the section scrolls past, the selection walks down the layers.
  useEffect(() => {
    const node = root.current;
    if (!node || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    let frame = 0;
    const update = () => {
      frame = 0;
      const i = scrolledTo();
      if (i !== -1 && i !== lastScrolled.current) {
        lastScrolled.current = i;
        setIndex(i);
      }
    };
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update);
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      window.removeEventListener("scroll", onScroll);
      cancelAnimationFrame(frame);
    };
  }, []);

  // On a narrow screen the card shrinks to the canvas.
  useEffect(() => {
    const node = pane.current;
    if (!node || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry) setScale(Math.min(FULL, (entry.contentRect.width - 8) / 232));
    });
    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, []);

  // The selection box: measured on the drawn card, so it fits what is shown.
  useLayoutEffect(() => {
    const measure = () => {
      const host = canvas.current;
      const read = reads[index];
      const el = read && host?.querySelector(target[read.layer] ?? "");
      if (!host || !el) {
        setBox(null);
        return;
      }
      const a = host.getBoundingClientRect();
      const b = el.getBoundingClientRect();
      setBox({ left: b.left - a.left, top: b.top - a.top, width: b.width, height: b.height });
    };
    measure();
    window.addEventListener("resize", measure);
    return () => {
      window.removeEventListener("resize", measure);
    };
  }, [index, scale]);

  const read = reads[index] ?? reads[0];
  if (!read) return null;
  const { node } = read;
  const size = node.width !== undefined && node.height !== undefined ? `${node.width} × ${node.height}` : "";

  return (
    <div ref={root} className="relative mt-14 motion-safe:lg:h-[260vh]">
      <div className="grid grid-cols-4 motion-safe:lg:sticky motion-safe:lg:top-[calc(50vh-330px)] lg:grid-cols-8">
        <div className="col-span-4 overflow-hidden rounded-2xl border border-line bg-[#1b1d23] lg:col-span-6 lg:col-start-2 lg:grid lg:h-[660px] lg:grid-cols-[200px_minmax(0,1fr)_minmax(0,340px)]">
          {/* Layers */}
          <div className="border-b border-line p-3 lg:border-r lg:border-b-0">
            <p className="micro px-2 pb-2 text-muted">Layers</p>
            <div
              role="group"
              aria-label="Layers of the order card"
              className="flex gap-1 overflow-x-auto lg:flex-col lg:overflow-visible"
            >
              {reads.map((r, i) => (
                <button
                  key={r.layer}
                  type="button"
                  aria-pressed={i === index}
                  onClick={() => {
                    pick(i);
                  }}
                  onFocus={() => {
                    pick(i);
                  }}
                  className={cn(
                    "flex flex-none cursor-pointer items-baseline gap-2 rounded-md px-2 py-1.5 text-left text-[14px] whitespace-nowrap focus-visible:outline-2 focus-visible:outline-[#0D99FF]",
                    i === index ? "bg-[#0D99FF]/25 text-paper" : "text-paper/70 hover:bg-paper/5",
                  )}
                  style={{ paddingLeft: `${8 + (depth[r.layer] ?? 0) * 14}px` }}
                >
                  <span className="micro w-14 text-paper/65">{r.node.type}</span>
                  {r.layer}
                </button>
              ))}
            </div>
          </div>

          {/* Canvas */}
          <div className="grid min-h-[440px] place-items-center lg:min-h-[520px] bg-[#26282e] bg-[radial-gradient(rgb(246_244_238/0.07)_1px,transparent_1px)] [background-size:16px_16px] p-6">
            <div ref={pane} className="grid w-full place-items-center">
              <div ref={canvas} className="relative">
                <div style={{ width: 232 * scale, height: 278 * scale }}>
                  <div style={{ transform: `scale(${scale})`, transformOrigin: "top left" }}>
                    <OrderCard mode="light" />
                  </div>
                </div>
                {box && (
                  <div
                    aria-hidden="true"
                    className="pointer-events-none absolute border-[1.5px] border-[#0D99FF] transition-all duration-300 ease-out"
                    style={{ left: box.left, top: box.top, width: box.width, height: box.height }}
                  >
                    {[
                      "-top-[4px] -left-[4px]",
                      "-top-[4px] -right-[4px]",
                      "-bottom-[4px] -left-[4px]",
                      "-right-[4px] -bottom-[4px]",
                    ].map((at) => (
                      <span
                        key={at}
                        className={cn("absolute size-[7px] border border-[#0D99FF] bg-white", at)}
                      />
                    ))}
                    <span className="absolute -top-6 left-[-1.5px] rounded-[3px] bg-[#0A6FD1] px-1.5 py-0.5 text-[11px] leading-4 font-medium whitespace-nowrap text-white">
                      {node.name}
                    </span>
                    {size && (
                      <span className="absolute -bottom-6 left-1/2 -translate-x-1/2 rounded-[3px] bg-[#0A6FD1] px-1.5 py-0.5 text-[11px] leading-4 font-medium whitespace-nowrap text-white">
                        {size}
                      </span>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* What grafta answers */}
          <div className="flex min-h-0 flex-col border-t border-line lg:border-t-0 lg:border-l">
            <p className="micro flex items-center justify-between border-b border-line px-4 py-3 text-muted">
              <span>
                <span className="text-bud">read_node</span> · {node.id}
              </span>
            </p>
            <div
              className="max-h-[420px] min-h-0 flex-1 overflow-auto px-4 py-3 lg:max-h-none"
              role="region"
              aria-label="What read_node answers"
              // A region that scrolls is reached by the keyboard too.
              // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex
              tabIndex={0}
              aria-live="polite"
            >
              <JsonView value={node} />
            </div>
          </div>
        </div>
        <p className="micro col-span-4 mt-4 text-muted lg:col-span-6 lg:col-start-2">
          Recorded from grafta on the card's file. Values bound to a variable name it, in Bud.
        </p>
      </div>
    </div>
  );
}
