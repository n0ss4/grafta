import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HOLD_MS, STEP_MS, useTalkPlayer } from "./useTalkPlayer";

// Three requests of 2, 1 and 3 steps: a request is answered once its steps
// are shown, and its reply after them.
const steps = [2, 1, 3];

function reducedMotion(reduce: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduce && query.includes("reduce"),
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

describe("the hero's player", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("starts with the first request answered, so the first paint is complete", () => {
    reducedMotion(false);
    const { result } = renderHook(() => useTalkPlayer(steps));
    expect(result.current.active).toBe(0);
    expect(result.current.answered).toBe(true);
  });

  it("plays the next requests by itself, step by step", () => {
    reducedMotion(false);
    const { result } = renderHook(() => useTalkPlayer(steps));
    act(() => {
      vi.advanceTimersByTime(HOLD_MS);
    });
    expect(result.current.active).toBe(1);
    expect(result.current.shown).toBe(0);
    act(() => {
      vi.advanceTimersByTime(STEP_MS);
    });
    expect(result.current.shown).toBe(1);
    expect(result.current.answered).toBe(true);
    act(() => {
      vi.advanceTimersByTime(HOLD_MS + STEP_MS * 3);
    });
    expect(result.current.active).toBe(2);
  });

  it("waits on the visitor once they pick a request", () => {
    reducedMotion(false);
    const { result } = renderHook(() => useTalkPlayer(steps));
    act(() => {
      result.current.pick(2);
    });
    expect(result.current.active).toBe(2);
    // Each step's timer is set once the last one has shown: one act a step.
    for (let tick = 0; tick < 20; tick++) {
      act(() => {
        vi.advanceTimersByTime(STEP_MS);
      });
    }
    act(() => {
      vi.advanceTimersByTime(HOLD_MS * 3);
    });
    expect(result.current.active).toBe(2);
    expect(result.current.answered).toBe(true);
  });

  it("under reduced motion, never moves by itself and shows answers whole", () => {
    reducedMotion(true);
    const { result } = renderHook(() => useTalkPlayer(steps));
    act(() => {
      vi.advanceTimersByTime(HOLD_MS * 10);
    });
    expect(result.current.active).toBe(0);
    act(() => {
      result.current.pick(1);
    });
    expect(result.current.answered).toBe(true);
    expect(result.current.shown).toBe(1);
  });
});

describe("a reply's words", () => {
  it("shows code inside bold as code, without its backticks", async () => {
    const { render } = await import("@testing-library/react");
    const { Rich } = await import("../Rich");
    const { container } = render(<Rich text={"saved as **`order-light.fig`**."} />);
    expect(container.querySelector("b > code")?.textContent).toBe("order-light.fig");
    expect(container.textContent).toBe("saved as order-light.fig.");
  });
});
