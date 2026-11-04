import { expect, test } from "@playwright/test";

test("from the top, Install then Copy puts the command on the clipboard (two actions)", async ({
  page,
  context,
  browserName,
}) => {
  test.skip(browserName !== "chromium", "clipboard permissions");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("./");
  await page.getByRole("navigation", { name: "Site" }).getByRole("link", { name: "Install" }).click();
  await page
    .locator("#install")
    .getByRole("tabpanel", { name: "Claude Code" })
    .getByRole("button", { name: "Copy" })
    .click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "/plugin install grafta --marketplace n0ss4/grafta",
  );
});

test("a client's tab, with its mark, picks its configuration", async ({ page }) => {
  await page.goto("./");
  await page.locator("#install").getByRole("tab", { name: "Cursor" }).click();
  await expect(page.locator("#install").getByRole("tab", { name: "Cursor" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await expect(page.locator("#install").getByText(".cursor/mcp.json")).toBeVisible();
});
