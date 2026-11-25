import { expect, test } from "@playwright/test";

// internal:specs/104: getting grafta, from the page.

test("the section offers the apps in two groups, Claude Desktop first, and its download", async ({
  page,
}) => {
  await page.goto("./");
  const get = page.locator("#get");
  await expect(get.getByRole("group", { name: "In your AI app" }).getByRole("button")).toHaveCount(7);
  await expect(
    get.getByRole("group", { name: "Or as a program, for any app" }).getByRole("button"),
  ).toHaveCount(4);
  await expect(get.getByRole("button", { name: "Claude Desktop" })).toHaveAttribute("aria-pressed", "true");
  await expect(
    get.getByRole("region", { name: "Install it in Claude Desktop" }).getByRole("link", { name: "Download" }),
  ).toHaveAttribute("href", "https://github.com/n0ss4/grafta/releases/latest/download/grafta.mcpb");
});

test("choosing an app shows its step, and one press copies it", async ({ page, context, browserName }) => {
  test.skip(browserName !== "chromium", "clipboard permissions");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("./");
  const get = page.locator("#get");
  await get.getByRole("button", { name: "Cursor" }).click();
  await expect(get.getByText(".cursor/mcp.json (or ~/.cursor/mcp.json)")).toBeVisible();
  // a long configuration scrolls in its box, never the page
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth),
  ).toBeLessThanOrEqual(0);
  await get.getByRole("button", { name: "Copy configuration" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain('"grafta"');
  await get.getByRole("button", { name: "Claude Code" }).click();
  await get.getByRole("button", { name: "Copy line" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "/plugin install grafta --marketplace n0ss4/grafta",
  );
});

test("the panel's step says each step, with its download and Figma's menus", async ({ page }) => {
  await page.goto("./");
  const get = page.locator("#get");
  await expect(get.getByRole("link", { name: "Download" }).nth(1)).toHaveAttribute(
    "href",
    "https://github.com/n0ss4/grafta/releases/latest/download/grafta-figma-plugin.zip",
  );
  for (const text of ["Import plugin from manifest…", "manifest.json", "Allow"]) {
    await expect(get).toContainText(text);
  }
});

test("the last step offers the first requests", async ({ page }) => {
  await page.goto("./");
  await expect(page.locator("#get").getByRole("region", { name: "Try it in Figma" })).toContainText(
    "Using grafta, what have I selected in Figma?",
  );
  const step = page.locator("#get").getByRole("region", { name: "Try it on a Figma file" });
  await expect(step).toContainText("Using grafta, what's in design.fig?");
  // in Claude Desktop, where the file is to be found
  await expect(step).toContainText("~/Downloads/design.fig");
});

test("every Get grafta leads to the section, with no dialog", async ({ page }) => {
  await page.goto("./");
  await page.locator("#get").getByRole("button", { name: "OpenCode" }).click();
  await page.locator("#hero").getByRole("link", { name: "Get grafta" }).click();
  await expect(page.locator("#get-title")).toBeInViewport();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator("#get").getByRole("button", { name: "OpenCode" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.goto("tools/");
  await page.getByRole("navigation", { name: "Site" }).getByRole("link", { name: "Get grafta" }).click();
  await expect(page.locator("#get-title")).toBeInViewport();
});

test("at 1440×900 the first step holds the apps and the chosen one's step in one screen", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("./");
  await page.evaluate(() => {
    document.querySelector("#get-title")?.scrollIntoView({ block: "start" });
  });
  await expect(
    page.locator("#get").getByRole("region", { name: "Install it in Claude Desktop" }),
  ).toBeInViewport({
    ratio: 1,
  });
});
