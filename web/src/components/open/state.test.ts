import { describe, expect, it } from "vitest";
import type { Summary } from "./protocol";
import { archiveName, MAX_BYTES, pngName, reduce, unique, visiblePages, type State } from "./state";

const summary: Summary = {
  file: "order.fig",
  handle: "order-1",
  product: "design",
  nodes: 23,
  pages: [
    { id: "1:0", name: "Showcase", nodes: 21 },
    { id: "1:9", name: "Internal Only Canvas", nodes: 1, internalOnly: true },
  ],
  variables: 9,
  variable_collections: 1,
};

describe("the page's states", () => {
  it("loads grafta first, then reads, then shows the file", () => {
    let state: State = { kind: "empty" };
    state = reduce(state, { type: "chose", name: "order.fig", bytes: 28_511, readerReady: false });
    expect(state).toEqual({ kind: "loading", name: "order.fig" });
    state = reduce(state, { type: "ready" });
    expect(state).toEqual({ kind: "reading", name: "order.fig" });
    state = reduce(state, { type: "opened", summary });
    expect(state.kind).toBe("read");
  });

  it("reads at once when grafta is already loaded", () => {
    expect(
      reduce({ kind: "empty" }, { type: "chose", name: "a.fig", bytes: 1, readerReady: true }).kind,
    ).toBe("reading");
  });

  it("refuses a file over the bound before reading, and an empty one", () => {
    const over = reduce(
      { kind: "empty" },
      { type: "chose", name: "big.fig", bytes: MAX_BYTES + 1, readerReady: true },
    );
    expect(over.kind).toBe("refused");
    expect(over.kind === "refused" && over.message).toContain("512 MB");
    expect(
      reduce({ kind: "empty" }, { type: "chose", name: "e.fig", bytes: 0, readerReady: true }).kind,
    ).toBe("refused");
  });

  it("says why a read failed, and goes back to empty when stopped", () => {
    const failed = reduce(
      { kind: "reading", name: "x.txt" },
      { type: "failed", message: "not a Figma file" },
    );
    expect(failed).toEqual({ kind: "refused", name: "x.txt", message: "not a Figma file" });
    expect(reduce({ kind: "reading", name: "x" }, { type: "stopped" })).toEqual({ kind: "empty" });
  });

  it("ignores an answer that arrives after a stop", () => {
    expect(reduce({ kind: "empty" }, { type: "opened", summary })).toEqual({ kind: "empty" });
  });

  it("hides Figma's own pages and counts them", () => {
    const { pages, internal } = visiblePages(summary);
    expect(pages.map((p) => p.name)).toEqual(["Showcase"]);
    expect(internal).toBe(1);
  });
});

describe("file names", () => {
  it("names a picture after its screen, with its scale", () => {
    expect(pngName("Order", 1)).toBe("Order.png");
    expect(pngName("Home / Desktop", 2)).toBe("Home _ Desktop@2x.png");
    expect(pngName("  ", 1)).toBe("screen.png");
  });

  it("keeps names apart within an archive", () => {
    expect(unique(["Frame.png", "Frame.png", "Card.png", "Frame.png"])).toEqual([
      "Frame.png",
      "Frame 2.png",
      "Card.png",
      "Frame 3.png",
    ]);
  });

  it("names an archive after the file and what it holds", () => {
    expect(archiveName("order.fig", "Showcase")).toBe("order – Showcase.zip");
  });
});
