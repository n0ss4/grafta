import { useEffect, useRef, useState } from "react";

/**
 * Whether an element has come into view, once: the section's entry motion
 * starts then. True at build time and without IntersectionObserver, so the
 * pre-rendered page shows everything.
 */
export function useInView<T extends Element>(threshold = 0.2) {
  const ref = useRef<T>(null);
  const [seen, setSeen] = useState(false);
  useEffect(() => {
    const node = ref.current;
    if (!node || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) {
          setSeen(true);
          observer.disconnect();
        }
      },
      { threshold },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [threshold]);
  return { ref, seen };
}
