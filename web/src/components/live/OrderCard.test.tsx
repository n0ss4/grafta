import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { OrderCard } from "./OrderCard";

describe("OrderCard", () => {
  it("resolves the Theme in the mode it is given, as the Figma component does", () => {
    const { container } = render(<OrderCard mode="hc" />);
    expect(container.firstElementChild).toHaveClass("mode-hc");
    expect(container.firstElementChild).toHaveAttribute("data-node", "FRAME Order");
    expect(screen.getByText("Track order")).toBeVisible();
  });

  it("keeps a hidden layer's place, as the build sequence shows it", () => {
    render(<OrderCard mode="light" hidden={["button"]} />);
    expect(screen.getByText("Track order").closest('[data-node="FRAME Button"]')).toHaveClass("invisible");
    expect(screen.getByText("Your order")).not.toHaveClass("invisible");
  });
});
