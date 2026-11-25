import { expect, test } from "@playwright/test";

// The first screen: the line, the button, and the product at work beginning under them.
const sizes = [
  [1280, 720],
  [1366, 768],
  [1440, 900],
  [1536, 864],
  [1920, 1080],
  [768, 1024],
  [390, 844],
] as const;

for (const [width, height] of sizes) {
  test(`at ${width}×${height} the line, the button and the product's window are in the first screen`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height });
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("./");
    await expect(page.getByRole("heading", { level: 1 })).toBeInViewport({ ratio: 1 });
    await expect(page.locator("#hero").getByRole("link", { name: "Get grafta" })).toBeInViewport({
      ratio: 1,
    });
    await expect(
      page.locator("#hero").getByRole("img", { name: /changes the play button/ }),
    ).toBeInViewport();
  });
}

test("the line is the one agreed", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Talk to your Figma.");
  await expect(page.locator("#hero")).toContainText("grafta connects your AI to Figma.");
});

test("under reduced motion the window shows the change made, and stays", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  const window = page.locator("#hero").getByRole("img", { name: /changes the play button/ });
  await expect(window).toContainText("Done. The play button is round");
  await page.waitForTimeout(3500);
  await expect(window).toContainText("Done. The play button is round");
  await expect(window.getByText("36", { exact: true })).toBeVisible();
});

test("what is new leads to the panel's section", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("./");
  await page
    .locator("#hero")
    .getByRole("link", { name: /Now in the Figma file you have open/ })
    .click();
  await expect(page.locator("#live-title")).toBeInViewport();
});

test("the pictures are served", async ({ page, request }) => {
  await page.goto("./");
  const sources = await page
    .locator("main img")
    .evaluateAll((nodes) => [...new Set(nodes.map((node) => (node as HTMLImageElement).src))]);
  expect(sources.length).toBeGreaterThanOrEqual(4);
  for (const source of sources) expect((await request.get(source)).ok()).toBe(true);
});

test("picking a request shows its answer", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  const requests = page.getByRole("group", { name: "Requests" });
  await requests.getByRole("button", { name: "Can every text be read?" }).click();
  await expect(requests.getByRole("button", { name: "Can every text be read?" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.locator("#ask")).toContainText("all five pass WCAG AA");
  await requests.getByRole("button", { name: "Make it light." }).click();
  await expect(
    page.locator("#ask").getByRole("img", { name: "The same design in its light mode" }),
  ).toBeVisible();
});
