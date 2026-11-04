import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

// WCAG 2.2 AA on every page, light and dark.
for (const scheme of ["light", "dark"] as const) {
  for (const path of ["./", "tools/", "changelog/", "404.html"]) {
    test(`${path} in ${scheme}`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await page.goto(path);
      // The hero's client with a row of calls open.
      if (path === "./") {
        const hero = page.locator("#client");
        await hero
          .getByRole("button", { name: /Called grafta/ })
          .first()
          .click();
        await hero.getByRole("button", { name: /Started a file the size of the picture/ }).click();
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
