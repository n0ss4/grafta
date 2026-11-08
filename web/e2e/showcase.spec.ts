import { expect, test } from "@playwright/test";

test("the page is the video, then what to know, how to get it, and questions", async ({ page }) => {
  await page.goto("./");
  const ids = await page.locator("main > [id]").evaluateAll((nodes) => nodes.map((node) => node.id));
  expect(ids).toEqual(["hero", "trust", "faq", "get"]);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Talk to your Figma files.");
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
