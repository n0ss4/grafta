import { expect, test } from "@playwright/test";

// internal:specs/105: a tool reference to find your way in.

test("every tool is one closed line, by family, with what it does and its arguments", async ({ page }) => {
  await page.goto("tools/");
  const tools = page.locator("details.tool");
  await expect(tools).toHaveCount(43);
  await expect(page.locator("details.tool[open]")).toHaveCount(0);
  const openFile = page.locator("#open_file summary");
  await expect(openFile).toContainText("Open a Figma .fig file and keep it open for later calls.");
  await expect(openFile).toContainText("2 arguments");
});

for (const [width, height, most] of [
  [1440, 900, 6000],
  [390, 844, 9000],
] as const) {
  test(`at ${width}×${height} the closed reference is at most ${most} px long, and nothing is wider than the screen (SC-001, SC-004)`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height });
    await page.goto("tools/");
    expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBeLessThanOrEqual(most);
    await page.getByRole("button", { name: "Open all" }).click();
    await expect(page.locator("details.tool[open]")).toHaveCount(43);
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth),
    ).toBeLessThanOrEqual(0);
    await page.getByRole("button", { name: "Close all" }).click();
    await expect(page.locator("details.tool[open]")).toHaveCount(0);
  });
}

test("opening a tool shows its words and its arguments, required first", async ({ page }) => {
  await page.goto("tools/");
  await page.locator("#render_node summary").click();
  const args = page.locator("#render_node dl > div");
  await expect(args.first()).toContainText("required");
  await expect(page.locator("#render_node")).toContainText("Pixels per unit.");
});

test("search finds a tool by what it does, opens it, and counts (SC-002)", async ({ page }) => {
  await page.goto("tools/");
  const search = page.getByLabel("Find a tool by its name, an argument, or what it does");
  for (const word of ["picture", "colour", "contrast"]) {
    await search.fill(word);
    await expect(page.locator("details.tool").first()).toBeVisible();
  }
  await search.fill("picture");
  await expect(page.locator("#render_node")).toHaveAttribute("open", "");
  await expect(page.getByText(/of 43 tools/)).toBeVisible();
  await search.fill("fill");
  await expect(page.locator("#fill_template")).toBeVisible();
  await expect(page.locator("#summarize")).toHaveCount(0);
  await search.fill("zzzz");
  await expect(page.getByText("Clear the search to see all 43.")).toBeVisible();
});

test("a link to one tool opens it, its name below the nav and the search (SC-003)", async ({ page }) => {
  await page.goto("tools/#set_properties");
  const tool = page.locator("#set_properties");
  await expect(tool).toHaveAttribute("open", "");
  await expect
    .poll(async () => {
      const name = await tool.locator("summary").boundingBox();
      const bar = await page.getByRole("search").boundingBox();
      return name && bar ? name.y >= bar.y + bar.height - 1 && name.y < 300 : false;
    })
    .toBe(true);
});

test("the search stays at hand while scrolling", async ({ page }) => {
  await page.goto("tools/");
  await page.mouse.wheel(0, 3000);
  await expect(page.getByRole("search")).toBeInViewport();
});

test("on a desktop the side list marks the family in view", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("tools/#tokens");
  const side = page.getByRole("navigation", { name: "Families of tools" }).first();
  await expect(side.getByRole("link", { name: "Tokens" })).toHaveAttribute("aria-current", "location");
});

test("on a phone the families are links at the top", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("tools/");
  const row = page.getByRole("navigation", { name: "Families of tools" }).last();
  await expect(row.getByRole("link")).toHaveCount(10);
  await row.getByRole("link", { name: /Tokens/ }).click();
  await expect(page.locator("#tokens-title")).toBeInViewport();
});
