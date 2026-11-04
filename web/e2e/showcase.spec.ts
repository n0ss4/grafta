import { expect, test } from "@playwright/test";

// 02: Figma's panes, with what grafta's read_node answers.
test("picking a layer shows what read_node answers for it", async ({ page }) => {
  await page.goto("./");
  const layers = page.getByRole("group", { name: "Layers of the order card" }).getByRole("button");
  await expect(layers).toHaveCount(7);
  await layers.filter({ hasText: "Button" }).first().click();
  const answer = page.getByRole("region", { name: "What read_node answers" });
  await expect(answer).toContainText('"name": "Button"');
  await expect(answer).toContainText('"variable": "Accent"');
});

// 00: the hero's client plays sessions as they were recorded.
test("the hero plays the picture session to its end, ending with the file it wrote", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("./");
  const hero = page.locator("#client");
  await expect(hero.getByRole("log", { name: "Conversation" })).toBeVisible();
  await expect(hero.getByRole("button", { name: "Replay" })).toBeVisible({ timeout: 60_000 });
  await expect(hero.getByText(/· poster-\w+\.fig/)).toBeVisible();
  await expect(hero.getByText(/Recorded in Claude Code/)).toBeVisible();
});

test("with reduced motion, the session shows whole and a row opens to its calls", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  const hero = page.locator("#client");
  await expect(hero.getByRole("button", { name: "Replay" })).toBeVisible();
  await hero
    .getByRole("button", { name: /Called grafta/ })
    .first()
    .click();
  const row = hero.getByRole("button", { name: /Started a file the size of the picture/ });
  await row.click();
  await expect(row).toHaveAttribute("aria-expanded", "true");
  await expect(hero.getByRole("region", { name: "Calls" }).first()).toContainText("new_file");
});

test("every request is one action away, and plays in the client", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  const hero = page.locator("#client");
  for (const [tab, text] of [
    ["Theme it", "High contrast"],
    ["Audit it", "order.fig"],
    ["Fix contrast", "checkout.fig"],
    ["Fill from data", "4907"],
    ["Bring in a web page", "pricing.json"],
  ] as const) {
    await hero.getByRole("tab", { name: tab }).click();
    await expect(hero.getByRole("log", { name: "Conversation" })).toContainText(text);
  }
});

test("a job's Play opens its session in the hero", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  await page.locator("#jobs a", { hasText: "Fix contrast" }).click();
  await expect(page.locator("#client").getByRole("tab", { name: "Fix contrast" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
});

test("the same request without grafta and with it, both recorded", async ({ page }) => {
  await page.goto("./");
  const compare = page.locator("#compare");
  await expect(compare.getByRole("article", { name: "Without grafta" })).toContainText("You hold:");
  await expect(compare.getByRole("article", { name: "With grafta" })).toContainText(/order-\w+\.fig/);
});

for (const width of [375, 768, 1440]) {
  test(`at ${width} px a session plays without scrolling sideways`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("./");
    await page.waitForTimeout(3000);
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
