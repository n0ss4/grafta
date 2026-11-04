import { expect, test } from "@playwright/test";

test("the reference lists every tool and narrows as you type", async ({ page }) => {
  await page.goto("tools/");
  await expect(page.locator("article.tool")).toHaveCount(43);
  await page.getByLabel("Find a tool or an argument").fill("fill");
  await expect(page.locator("#fill_template")).toBeVisible();
  await expect(page.locator("#create_text")).toBeVisible();
  await expect(page.locator("#summarize")).toHaveCount(0);
  await expect(page.getByText(/of 43 tools/)).toBeVisible();
});

test("a link to one tool opens at it", async ({ page }) => {
  await page.goto("tools/#render_node");
  await expect(page.locator("#render_node")).toBeInViewport();
});
