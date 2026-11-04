import { expect, test } from "@playwright/test";

test("from the top, Get grafta then Copy line puts the command on the clipboard", async ({
  page,
  context,
  browserName,
}) => {
  test.skip(browserName !== "chromium", "clipboard permissions");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("./");
  await page.getByRole("navigation", { name: "Site" }).getByRole("button", { name: "Get grafta" }).click();
  const picker = page.getByRole("dialog", { name: "Get grafta" });
  await picker.getByRole("button", { name: "Copy line" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "/plugin install grafta --marketplace n0ss4/grafta",
  );
});

test("the picker lists every app and shows its file or line", async ({ page }) => {
  await page.goto("./");
  await page.locator("#hero").getByRole("button", { name: "Get grafta" }).click();
  const picker = page.getByRole("dialog", { name: "Get grafta" });
  await expect(picker.getByRole("button", { name: "Claude Code" })).toBeVisible();
  await picker.getByRole("button", { name: "Cursor" }).click();
  await expect(picker.getByText(".cursor/mcp.json (or ~/.cursor/mcp.json)")).toBeVisible();
});
