import { expect, test } from "@playwright/test";

// The old addresses keep answering.
for (const path of [
  "tools/",
  "changelog/",
  "install.sh",
  "install.ps1",
  "snapshot.js",
  "snapshot.sh",
  "assets/card.png",
  "brand/mark.svg",
  "robots.txt",
  "sitemap.xml",
]) {
  test(`${path} answers`, async ({ request }) => {
    const response = await request.get(path);
    expect(response.status()).toBe(200);
  });
}

test("the old reference's address lands on the new one", async ({ page }) => {
  await page.goto("tools.html#render_node");
  await expect(page).toHaveURL(/\/grafta\/tools\/#render_node$/);
  await expect(
    page.getByRole("heading", { name: /tools, as the server lists them/, level: 1 }),
  ).toBeVisible();
});

test("an unknown address gets the 404 page", async ({ page }) => {
  await page.goto("404.html");
  await expect(page.getByRole("heading", { name: "No node at this path." })).toBeVisible();
  await expect(page.locator('meta[name="robots"]')).toHaveAttribute("content", "noindex");
});
