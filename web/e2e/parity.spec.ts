import { expect, test } from "@playwright/test";

test("the home page has its content and metadata", async ({ page }) => {
  await page.goto("./");
  await expect(page).toHaveTitle("grafta · Your agent, on your Figma files");
  for (const product of ["Figma Design", "Figma Slides", "FigJam", "Figma Sites"]) {
    await expect(page.locator("#products").getByRole("heading", { name: product })).toBeVisible();
  }
  await expect(page.locator("#jobs li h3")).toHaveCount(6);
  await expect(page.locator("#faq details")).toHaveCount(10);
  await expect(page.locator("#why ol > li")).toHaveCount(4);
  await expect(page.locator("#limits li")).toHaveCount(7);
  await expect(page.locator("#difference tbody tr")).toHaveCount(6);

  const card = "https://n0ss4.github.io/grafta/assets/card.png";
  await expect(page.locator('meta[property="og:image"]')).toHaveAttribute("content", card);
  await expect(page.locator('meta[name="twitter:image"]')).toHaveAttribute("content", card);
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    "https://n0ss4.github.io/grafta/",
  );
  await expect(page.locator('script[type="application/ld+json"]')).toHaveCount(1);
});

test("the skip link leads to the content", async ({ page }) => {
  await page.goto("./");
  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "Skip to content" });
  await expect(skip).toBeFocused();
  await expect(skip).toHaveAttribute("href", "#main");
});
