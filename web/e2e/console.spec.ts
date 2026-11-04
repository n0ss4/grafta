import { expect, test } from "@playwright/test";

// Hydration that disagrees with the pre-rendered HTML redraws the page; none is allowed.
for (const path of ["./", "tools/", "changelog/"]) {
  test(`${path} hydrates without an error`, async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (message) => {
      if (message.type() === "error" || message.type() === "warning") errors.push(message.text());
    });
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(path);
    await page.waitForLoadState("networkidle");
    expect(errors).toEqual([]);
  });
}
