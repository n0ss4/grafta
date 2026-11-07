import { useCallback, useEffect, useState, useSyncExternalStore } from "react";

/** How long one of grafta's steps takes to appear. */
export const STEP_MS = 650;
/** How long an answered request stays before the next plays by itself. */
export const HOLD_MS = 6500;

export interface TalkPlayer {
  /** The request shown. */
  active: number;
  /** How many of its steps are shown; the reply follows the last. */
  shown: number;
  /** Whether its steps and reply are all shown. */
  answered: boolean;
  /** Whether it plays by itself: until the visitor picks, never under reduced motion. */
  auto: boolean;
  pick: (index: number) => void;
}

const REDUCE = "(prefers-reduced-motion: reduce)";

/** Whether the visitor asks for reduced motion: false on the server, live in the browser. */
export function useReducedMotion(): boolean {
  return useSyncExternalStore(
    (changed) => {
      const query = window.matchMedia(REDUCE);
      query.addEventListener("change", changed);
      return () => {
        query.removeEventListener("change", changed);
      };
    },
    () => window.matchMedia(REDUCE).matches,
    () => false,
  );
}

/**
 * The hero's requests: the first starts answered, so the page paints complete
 * and still; then each plays step by step, and the next comes after a pause,
 * until the visitor picks one. Under reduced motion nothing moves: a request
 * picked is shown whole. `steps` is each request's number of steps.
 */
export function useTalkPlayer(steps: readonly number[]): TalkPlayer {
  const [active, setActive] = useState(0);
  const [shown, setShown] = useState(steps[0] ?? 0);
  const [picked, setPicked] = useState(false);
  const still = useReducedMotion();
  const auto = !picked && !still;

  const total = steps[active] ?? 0;
  const answered = shown >= total;

  useEffect(() => {
    if (still) return;
    if (!answered) {
      const timer = setTimeout(() => {
        setShown((n) => n + 1);
      }, STEP_MS);
      return () => {
        clearTimeout(timer);
      };
    }
    if (!auto) return;
    const timer = setTimeout(() => {
      setActive((index) => (index + 1) % steps.length);
      setShown(0);
    }, HOLD_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [shown, answered, auto, still, steps.length, active]);

  const pick = useCallback(
    (index: number) => {
      setPicked(true);
      setActive(index);
      setShown(still ? (steps[index] ?? 0) : 0);
    },
    [still, steps],
  );

  return { active, shown, answered, auto, pick };
}
