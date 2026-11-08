import { expect, test } from "@playwright/test";

test("home tells the complete story without JavaScript", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Talk to your Figma files.");
  // The video's six scenes, named and told, and its poster in place of it.
  const rail = page.getByRole("list", { name: "Scenes of the video" });
  await expect(rail.getByRole("listitem")).toHaveCount(6);
  for (const name of [
    "Your AI app",
    "Add a file",
    "Ask what's in it",
    "Change it, check it",
    "Turn it into code",
    "Open it in Figma",
  ]) {
    await expect(rail).toContainText(name);
  }
  await expect(page.locator("#hero-video-scenes li")).toHaveCount(6);
  await expect(page.locator("#hero-video-scenes")).toContainText("It saves a new file, never over yours.");
  await expect(page.locator("#hero video")).toHaveAttribute("poster", /poster\.jpg$/);
  await expect(page.locator("#trust")).toContainText("No account");
  // Getting grafta works without scripts: the first app's step, and links to it.
  await expect(page.locator("#get")).toContainText("Install it in Claude Desktop");
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
  await expect(page.locator("article.tool")).toHaveCount(43);
});

test("changelog", async ({ page }) => {
  await page.goto("changelog/");
  await expect(page.getByRole("heading", { name: "0.1.0" })).toBeVisible();
});
