import { expect, test } from "@playwright/test";

test("the home page keeps its products, questions and metadata", async ({ page }) => {
  await page.goto("./");
  await expect(page).toHaveTitle("grafta · Ask your AI. Get a Figma file back.");
  for (const extension of [".fig", ".deck", ".jam", ".site"]) {
    await expect(page.locator("#trust").getByText(extension, { exact: true })).toBeVisible();
  }
  await expect(page.locator("#faq details")).toHaveCount(11);
  await expect(page.locator("#difference article").first().locator("dl > div")).toHaveCount(6);
  await expect(
    page.getByRole("navigation", { name: "Product" }).getByRole("link", { name: "Get grafta" }),
  ).toHaveAttribute("href", "#get");
  await expect(
    page.getByRole("navigation", { name: "Product" }).getByRole("link", { name: "Examples" }),
  ).toHaveAttribute("href", "#ask");
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
