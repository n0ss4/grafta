import { render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Comparison } from "./Comparison";
import type { Recording } from "./types";

const read = (name: string) =>
  JSON.parse(readFileSync(join(process.cwd(), "public", "sessions", `${name}.json`), "utf8")) as Recording;

describe("Comparison", () => {
  it("shows the same prompt answered without and with grafta, each ending with what the person holds", () => {
    const without = read("theme-without.claude-code");
    const withGrafta = read("theme.claude-code");
    render(<Comparison without={without} withGrafta={withGrafta} />);
    const [left, right] = screen.getAllByRole("article");
    if (!left || !right) throw new Error("two articles expected");
    expect(left).toHaveAccessibleName("Without grafta");
    expect(right).toHaveAccessibleName("With grafta");
    const last = without.turns.at(-1);
    expect(within(left).getByText(/You hold:/).parentElement).toHaveTextContent("the file untouched");
    expect(left).toHaveTextContent(last?.kind === "reply" ? last.text.slice(0, 30) : "");
    expect(within(right).getByText(/You hold:/).parentElement).toHaveTextContent(withGrafta.file ?? "");
  });
});
