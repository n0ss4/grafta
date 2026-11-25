import { expect, test } from "@playwright/test";

test("the page is the product at work, then what it does, the panel, what to ask, questions, how to get it and the last word", async ({
  page,
}) => {
  await page.goto("./");
  const ids = await page.locator("main > [id]").evaluateAll((nodes) => nodes.map((node) => node.id));
  expect(ids).toEqual(["hero", "files", "what", "live", "ask", "faq", "get", "closing"]);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Talk to your Figma.");
});

for (const width of [375, 768, 1440]) {
  test(`at ${width} px the new page does not scroll sideways`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("./");
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(sideways).toBeLessThanOrEqual(0);
  });
}

test("the colour switch shows both modes, starts on Ink whatever the system, and keeps the choice", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("./");
  const modes = page.getByRole("group", { name: "Colour mode" });
  await expect(modes.getByRole("button", { name: "Dark", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe("rgb(6, 6, 7)");
  await modes.getByRole("button", { name: "Light", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(modes.getByRole("button", { name: "Light", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.goto("tools/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute("content", "#FAFAFB");
});
