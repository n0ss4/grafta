import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { clients } from "~/content/clients";
import { Configurator } from "./Configurator";

function Harness() {
  const [value, setValue] = useState("claude-code");
  return <Configurator highlighted={{}} value={value} onValueChange={setValue} />;
}

describe("Configurator", () => {
  it("moves between clients with the arrow keys", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const first = screen.getByRole("tab", { name: "Claude Code" });
    await user.click(first);
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Codex" })).toHaveAttribute("aria-selected", "true");
    await user.keyboard("{End}");
    expect(screen.getByRole("tab", { name: "Rust" })).toHaveFocus();
  });

  it("copies the selected client's snippet exactly", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    render(<Harness />);
    await user.click(screen.getByRole("tab", { name: "Cursor" }));
    const panel = screen.getByRole("tabpanel", { name: "Cursor" });
    await user.click(panel.querySelector("button") as HTMLButtonElement);
    expect(writeText).toHaveBeenCalledWith(clients.find((c) => c.id === "cursor")?.snippet);
  });
});
