import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ToolRow } from "./ToolRow";

const calls = [
  { tool: "open_file", args: { path: "order.fig" }, result: '{"nodes": 23}' },
  {
    tool: "list_variables",
    args: { handle: "h" },
    result: '{"collections": []}',
    omitted: "…cut here: 12 more characters",
  },
];

describe("ToolRow", () => {
  it("is a closed row with its words and how many calls it covers", () => {
    render(<ToolRow label="Read the file" calls={calls} />);
    const row = screen.getByRole("button", { name: /Read the file/ });
    expect(row).toHaveAttribute("aria-expanded", "false");
    expect(row).toHaveTextContent("grafta");
    expect(row).toHaveTextContent("2 calls");
    expect(screen.queryByRole("region", { name: "Calls" })).toBeNull();
  });

  it("opens to each call as recorded, without the handle, and says what was cut", async () => {
    render(<ToolRow label="Read the file" calls={calls} />);
    await userEvent.click(screen.getByRole("button", { name: /Read the file/ }));
    const region = screen.getByRole("region", { name: "Calls" });
    expect(region).toHaveTextContent('open_file {"path":"order.fig"}');
    expect(region).toHaveTextContent('"nodes": 23');
    expect(region).toHaveTextContent("list_variables { }");
    expect(region).toHaveTextContent("…cut here: 12 more characters");
  });

  it("names the client's own tools as another tool, not grafta", () => {
    render(<ToolRow label="Used another tool" calls={[{ tool: "Bash", args: {}, result: "" }]} other />);
    const row = screen.getByRole("button", { name: /Used another tool/ });
    expect(row).not.toHaveTextContent("grafta");
  });
});
