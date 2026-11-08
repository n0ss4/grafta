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

test("the second step offers the sample and the first request", async ({ page, request }) => {
  await page.goto("./");
  const step = page.locator("#get").getByRole("region", { name: "Try it on a Figma file" });
  await expect(step).toContainText("Using grafta, what's in order.fig?");
  // in Claude Desktop, where the file is to be found
  await expect(step).toContainText("~/Downloads/order.fig");
  const sample = await step
    .getByRole("link", { name: "Download" })
    .evaluate((node) => (node as HTMLAnchorElement).href);
  expect(sample).toMatch(/sample\/order\.fig$/);
  expect((await request.get(sample)).ok()).toBe(true);
});

test("every Get grafta opens the same installer, with the app chosen in the section", async ({ page }) => {
  await page.goto("./");
  await page.locator("#get").getByRole("button", { name: "Windsurf" }).click();
  await page.getByRole("navigation", { name: "Site" }).getByRole("link", { name: "Get grafta" }).click();
  const dialog = page.getByRole("dialog", { name: "Get grafta" });
  await expect(dialog.getByRole("button", { name: "Windsurf" })).toHaveAttribute("aria-pressed", "true");
  await expect(dialog.getByText("~/.codeium/windsurf/mcp_config.json")).toBeVisible();
  await dialog.getByRole("button", { name: "Close" }).click();
  await page.locator("#hero").getByRole("link", { name: "Get grafta" }).click();
  await expect(dialog).toBeVisible();
});

test("at 1440×900 the section holds the apps, the step and the try step in one screen", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("./");
  await page.locator("#get-title").scrollIntoViewIfNeeded();
  await page.evaluate(() => {
    document.querySelector("#get-title")?.scrollIntoView({ block: "start" });
  });
  for (const name of ["Install it in Claude Desktop", "Try it on a Figma file"]) {
    await expect(page.locator("#get").getByRole("region", { name })).toBeInViewport({ ratio: 1 });
  }
});
