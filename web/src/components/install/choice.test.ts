import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { choose, resetChoice, useChoice } from "./choice";

describe("the chosen way in", () => {
  afterEach(() => {
    window.sessionStorage.clear();
    resetChoice();
  });

  it("is Claude Desktop until the visitor chooses", () => {
    const { result } = renderHook(() => useChoice());
    expect(result.current).toBe("claude-desktop");
  });

  it("is shared by every place that shows it, and kept for the visit", () => {
    const section = renderHook(() => useChoice());
    const dialog = renderHook(() => useChoice());
    act(() => {
      choose("cursor");
    });
    expect(section.result.current).toBe("cursor");
    expect(dialog.result.current).toBe("cursor");
    resetChoice();
    expect(renderHook(() => useChoice()).result.current).toBe("cursor");
  });

  it("ignores a way in it does not know", () => {
    act(() => {
      choose("netscape");
    });
    expect(renderHook(() => useChoice()).result.current).toBe("claude-desktop");
  });
});
