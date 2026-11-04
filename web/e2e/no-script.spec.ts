import { expect, test } from "@playwright/test";

// Run with JavaScript off: every page's content is in its HTML.
test("home", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { level: 1 })).toContainText(
    "Ask your agent. Get a Figma file back.",
  );
  const hero = page.locator("#client");
  await expect(hero.getByRole("log", { name: "Conversation" })).toContainText(
    "turn poster.png into a Figma file",
  );
  await expect(hero.getByText(/· poster-\w+\.fig/)).toBeVisible();
  await expect(page.locator("#compare article")).toHaveCount(2);
  await expect(page.locator("#faq details")).toHaveCount(8);
  await page.locator("#faq summary").first().click();
  await expect(page.locator("#faq details").first()).toHaveAttribute("open", "");
});

test("tools", async ({ page }) => {
  await page.goto("tools/");
  await expect(page.locator("article.tool")).toHaveCount(43);
});

test("changelog", async ({ page }) => {
  await page.goto("changelog/");
  await expect(page.getByRole("heading", { name: "0.1.0" })).toBeVisible();
});
