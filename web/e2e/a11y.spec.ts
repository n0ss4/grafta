import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

// WCAG 2.2 AA on every page, light and dark.
for (const scheme of ["light", "dark"] as const) {
  for (const path of ["./", "open/", "tools/", "changelog/", "404.html"]) {
    test(`${path} in ${scheme}`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await page.goto(path);
      // Exercise the modal as well as the page when JavaScript is available.
      if (path === "./") {
        await page.locator("#hero").getByRole("link", { name: "Get grafta" }).click();
      }
      // And a file open, with a screen selected and its tokens.
      if (path === "open/") {
        await page.getByRole("button", { name: /Open the sample/ }).click();
        await page.locator("li img").first().waitFor();
        await page.getByRole("button", { name: /^Order,/ }).click();
        await page.getByRole("img", { name: /drawn by grafta/ }).waitFor();
      }
      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
        .analyze();
      expect(
        results.violations.map(
          (v) =>
            `${v.id}: ${v.nodes
              .map((n) => n.target.join(" "))
              .slice(0, 3)
              .join(" | ")}`,
        ),
      ).toEqual([]);
    });
  }
}
