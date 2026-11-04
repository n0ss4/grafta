import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Client } from "./Client";
import type { Recording } from "./types";

const recordingOf = (name: string) =>
  JSON.parse(readFileSync(join(process.cwd(), "public", "sessions", `${name}.json`), "utf8")) as Recording;
const bundled = recordingOf("image.claude-code");

/** matchMedia as jsdom lacks it: reduced motion as asked, a phone when asked. */
function media(reduce: boolean, phone = false) {
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) => ({
      matches: query.includes("reduce") ? reduce : query.includes("max-width") ? phone : false,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    })),
  );
}

beforeEach(() => {
  media(true);
  Element.prototype.scrollIntoView = vi.fn();
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string) => {
      const name = /sessions\/(.+)\.json$/.exec(url)?.[1] ?? "";
      return Promise.resolve({ ok: true, json: () => Promise.resolve(recordingOf(name)) });
    }),
  );
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Client", () => {
  it("shows the bundled session whole under reduced motion, ending with its file and where it was recorded", () => {
    render(<Client first={bundled} client="claude-code" onClient={() => undefined} />);
    const log = screen.getByRole("log", { name: "Conversation" });
    expect(within(log).getAllByRole("listitem").length).toBeGreaterThan(3);
    const end = bundled.turns.at(-1);
    expect(log).toHaveTextContent(end?.kind === "reply" ? end.text.slice(0, 40) : "");
    expect(screen.getByText(new RegExp(`· ${bundled.file ?? ""}`))).toBeInTheDocument();
    expect(screen.getByText(/Recorded in Claude Code .* with .* on \d{4}-\d{2}-\d{2}/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Replay" })).toBeInTheDocument();
  });

  it("folds each run of work into one line, as Claude Code does, and opens it to its rows and calls", async () => {
    render(<Client first={bundled} client="claude-code" onClient={() => undefined} />);
    const runs = screen.getAllByRole("button", { name: /Called grafta/ });
    expect(runs.length).toBeGreaterThan(1);
    expect(screen.queryByRole("button", { name: /Started a file the size of the picture/ })).toBeNull();
    const [first] = runs;
    if (first) await userEvent.click(first);
    const [row] = screen.getAllByRole("button", { name: /Started a file the size of the picture/ });
    if (row) await userEvent.click(row);
    expect(screen.getAllByRole("region", { name: "Calls" })[0]).toHaveTextContent("new_file");
  });

  it("plays from the prompt, step by step, when asked", async () => {
    render(<Client first={bundled} client="claude-code" onClient={() => undefined} />);
    await userEvent.click(screen.getByRole("button", { name: "Replay" }));
    await userEvent.click(screen.getByRole("button", { name: "Pause" }));
    await userEvent.click(screen.getByRole("button", { name: "Step" }));
    const items = within(screen.getByRole("log", { name: "Conversation" })).getAllByRole("listitem");
    expect(items.length).toBeLessThan(bundled.turns.length);
  });

  it("loads another request when its tab is picked, and plays it when a job asks for it", async () => {
    render(<Client first={bundled} client="claude-code" onClient={() => undefined} />);
    await userEvent.click(screen.getByRole("tab", { name: "Fix contrast" }));
    await waitFor(() => {
      expect(screen.getByRole("log", { name: "Conversation" })).toHaveTextContent("checkout.fig");
    });
    act(() => {
      window.dispatchEvent(new CustomEvent("grafta:session", { detail: "html" }));
    });
    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Bring in a web page" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });
    await waitFor(() => {
      expect(screen.getByRole("log", { name: "Conversation" })).toHaveTextContent("pricing.json");
    });
  });

  it("on a phone, draws the design inside the conversation, under the last row of calls", () => {
    media(true, true);
    render(<Client first={bundled} client="claude-code" onClient={() => undefined} />);
    const log = screen.getByRole("log", { name: "Conversation" });
    expect(within(log).getByText("poster.png · the picture")).toBeInTheDocument();
  });

  it("says when a session does not load, and tries again", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => Promise.resolve({ ok: false, status: 404 })),
    );
    render(<Client first={bundled} client="claude-code" onClient={() => undefined} />);
    fireEvent.click(screen.getByRole("tab", { name: "Audit it" }));
    expect(await screen.findByText("This session did not load.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Try again" })).toBeInTheDocument();
  });
});
