import { expect, test } from "@playwright/test";

test("home tells the complete story without JavaScript", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Talk to your Figma files.");
  // Every request of the hero answered, still.
  for (const words of [
    "what's in order.fig?",
    "Make it light, and check the contrast.",
    "Now turn it into code.",
  ]) {
    await expect(page.locator("#hero")).toContainText(words);
  }
  await expect(page.locator("#hero [data-talk-panel]:visible")).toHaveCount(3);
  await expect(page.locator("#hero")).toContainText("order-light.fig");
  await expect(page.locator("#how [data-demo-step]")).toHaveCount(6);
  await expect(page.locator("#how")).toContainText("poster-editable.fig");
  await expect(page.locator("#ask article")).toHaveCount(6);
  await expect(page.locator("#difference article")).toHaveCount(2);
  await expect(page.locator("#trust")).toContainText("No account");
  await expect(page.locator("#faq details")).toHaveCount(11);
});

test("tools", async ({ page }) => {
  await page.goto("tools/");
  await expect(page.locator("article.tool")).toHaveCount(43);
});

test("changelog", async ({ page }) => {
  await page.goto("changelog/");
  await expect(page.getByRole("heading", { name: "0.1.0" })).toBeVisible();
});
