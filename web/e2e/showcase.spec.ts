import { expect, test } from "@playwright/test";

test("the page follows the approved product story", async ({ page }) => {
  await page.goto("./");
  const ids = await page.locator("main > [id]").evaluateAll((nodes) => nodes.map((node) => node.id));
  expect(ids).toEqual(["hero", "how", "ask", "difference", "trust", "faq", "get"]);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Talk to your Figma files.");
  await expect(page.locator("#how [data-demo-step]")).toHaveCount(6);
  await expect(page.locator("#ask article")).toHaveCount(6);
  await expect(page.locator("#difference article")).toHaveCount(2);
});

test("the guided demo advances with arrow keys", async ({ page }) => {
  await page.goto("./");
  const first = page.locator("#how [data-demo-step]").first().getByRole("button");
  await first.focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.locator("#how [data-demo-step]").nth(1).getByRole("button")).toHaveAttribute(
    "aria-current",
    "step",
  );
  await page.keyboard.press("ArrowUp");
  await expect(first).toHaveAttribute("aria-current", "step");
});

test("reduced motion shows all six demo cards", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("./");
  await expect(
    page.locator("#how [data-demo-step]").first().locator("div.relative.min-h-\\[460px\\]"),
  ).toBeVisible();
  await expect(
    page.locator("#how [data-demo-step]").last().locator("div.relative.min-h-\\[460px\\]"),
  ).toBeAttached();
});

for (const width of [375, 768, 1440]) {
  test(`at ${width} px the new page does not scroll sideways`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("./");
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(sideways).toBeLessThanOrEqual(0);
  });
}

test("the theme toggle swaps Paper and Ink, and keeps the choice", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("./");
  await page.getByRole("button", { name: "Dark mode", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.goto("tools/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});
