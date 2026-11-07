import { expect, test, type Page } from "@playwright/test";

/** Whether an element's box is wholly inside the first viewport. */
async function inFirstScreen(page: Page, selector: string) {
  return page
    .locator(selector)
    .first()
    .evaluate((node) => {
      const box = node.getBoundingClientRect();
      return box.top >= 0 && box.bottom <= window.innerHeight && box.height > 0;
    });
}

for (const [name, size] of [
  ["a phone", { width: 390, height: 844 }],
  ["a desktop", { width: 1440, height: 900 }],
] as const) {
  test(`on ${name}, the first screen holds the file, a request and its answer`, async ({ page }) => {
    await page.setViewportSize(size);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("./");
    expect(await inFirstScreen(page, "#hero figure img[data-visible='true']")).toBe(true);
    expect(await inFirstScreen(page, "#hero [role=tabpanel]:not([hidden]) p")).toBe(true);
    const lead = page
      .locator(
        "#hero [role=tabpanel]:not([hidden]) [aria-live] p, #hero [role=tabpanel]:not([hidden]) [data-visible] > p",
      )
      .first();
    await expect(lead).toContainText("order.fig");
    expect(await lead.evaluate((node) => node.getBoundingClientRect().top < window.innerHeight)).toBe(true);
  });
}

test("picking a request shows its answer, on the same file", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  await page.getByRole("tab", { name: /Make it light/ }).click();
  const panel = page.locator("#hero [role=tabpanel]:not([hidden])");
  await expect(panel).toContainText("Make it light, and check the contrast.");
  await expect(panel).toContainText("order-light.fig");
  await expect(page.locator("#hero figcaption")).toHaveText("Drawn by grafta from order-light.fig");
  await expect(page.locator("#hero")).toContainText("Theme: Light mode");
});

test("the requests play by themselves, and stop once one is picked", async ({ page }) => {
  await page.clock.install();
  await page.goto("./");
  const selected = page.locator("#hero [role=tab][aria-selected=true]");
  await expect(selected).toContainText("What's in order.fig?");
  await page.clock.runFor(8000);
  await expect(selected).toContainText("Make it light");
  await page.getByRole("tab", { name: /Now turn it into code/ }).click();
  await page.clock.runFor(60000);
  await expect(selected).toContainText("Now turn it into code");
});
