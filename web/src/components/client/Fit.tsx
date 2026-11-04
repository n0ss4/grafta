import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

/** Scales what it holds down to fit its box, never up: the canvas fits one screen. */
export function Fit({ children }: { children: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  const inner = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  useLayoutEffect(() => {
    const outer = box.current;
    const content = inner.current;
    if (!outer || !content || typeof ResizeObserver === "undefined") return;
    const fit = () => {
      const w = content.scrollWidth;
      const h = content.scrollHeight;
      if (!w || !h) return;
      setScale(Math.min(1, (outer.clientWidth - 32) / w, (outer.clientHeight - 32) / h));
    };
    const observer = new ResizeObserver(fit);
    observer.observe(outer);
    observer.observe(content);
    fit();
    return () => {
      observer.disconnect();
    };
  }, []);
  return (
    <div ref={box} className="relative h-full w-full overflow-hidden">
      <div
        ref={inner}
        className="absolute top-1/2 left-1/2 w-max"
        style={{ transform: `translate(-50%, -50%) scale(${scale})` }}
      >
        {children}
      </div>
    </div>
  );
}
