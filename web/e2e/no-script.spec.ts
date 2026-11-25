import { expect, test } from "@playwright/test";

test("home tells the complete story without JavaScript", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Talk to your Figma.");
  // The product's window, drawn with the change made.
  const window = page.locator("#hero").getByRole("img", { name: /changes the play button/ });
  await expect(window).toContainText("Done. The play button is round");
  for (const extension of [".fig", ".deck", ".jam", ".site"]) {
    await expect(page.locator("#files")).toContainText(extension);
  }
  await expect(page.locator("#what")).toContainText("Reads your design");
  await expect(page.locator("#live")).toContainText("Revert this request");
  await expect(page.locator("#ask")).toContainText("Seven, each in light and dark.");
  // Getting grafta works without scripts: the first app's step, and links to it.
  await expect(page.locator("#get")).toContainText("Add grafta to your AI app");
  await expect(page.locator("#get").getByRole("link", { name: "Download" }).first()).toHaveAttribute(
    "href",
    /grafta\.mcpb$/,
  );
  await expect(page.locator("#hero").getByRole("link", { name: "Get grafta" })).toHaveAttribute(
    "href",
    "#get",
  );
  await expect(page.locator("#faq details")).toHaveCount(11);
});

test("tools", async ({ page }) => {
  await page.goto("tools/");
  await expect(page.locator("details.tool")).toHaveCount(45);
  // a tool opens without scripts
  await page.locator("#render_node summary").click();
  await expect(page.locator("#render_node")).toHaveAttribute("open", "");
  await expect(page.locator("#render_node")).toContainText("Pixels per unit.");
});

test("changelog", async ({ page }) => {
  await page.goto("changelog/");
  await expect(page.getByRole("heading", { name: "0.1.0" })).toBeVisible();
});

test("open says what it does, and how to read a file without scripts", async ({ page }) => {
  await page.goto("open/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Open a Figma file, without Figma.");
  // Chromium, with scripts off by a setting, keeps <noscript> as unparsed text, so the
  // page as served is read: a browser with scripts off shows this paragraph.
  const html = await (await page.request.get("open/")).text();
  expect(html).toContain("<noscript><p");
  expect(html).toContain("This page reads your file with JavaScript, which is off.");
});
