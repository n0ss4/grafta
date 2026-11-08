import { expect, test } from "@playwright/test";

for (const [name, size] of [
  ["a phone", { width: 390, height: 844 }],
  ["a desktop", { width: 1440, height: 900 }],
] as const) {
  test(`on ${name}, the first screen holds the line and the video`, async ({ page }) => {
    await page.setViewportSize(size);
    await page.goto("./");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("Talk to your Figma files.");
    const video = page.locator("#hero video");
    await expect(video).toBeVisible();
    const top = await video.evaluate((node) => node.getBoundingClientRect().top);
    expect(top).toBeLessThan(size.height);
  });
}

test("the video plays by itself, muted, with its controls", async ({ page }) => {
  await page.goto("./");
  const video = page.locator("#hero video");
  await expect(video).toHaveJSProperty("muted", true);
  await expect(video).toHaveAttribute("controls", "");
  await expect.poll(() => video.evaluate((node: HTMLVideoElement) => node.paused)).toBe(false);
});

test("under reduced motion the video waits for a click", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  const video = page.locator("#hero video");
  await expect(video).toBeVisible();
  await page.waitForTimeout(500);
  expect(await video.evaluate((node: HTMLVideoElement) => node.paused)).toBe(true);
});

test("its files are served", async ({ page, request }) => {
  await page.goto("./");
  for (const source of await page
    .locator("#hero video source")
    .evaluateAll((nodes) => nodes.map((node) => (node as HTMLSourceElement).src))) {
    expect((await request.get(source)).ok()).toBe(true);
  }
  const poster = await page.locator("#hero video").evaluate((node: HTMLVideoElement) => node.poster);
  expect((await request.get(poster)).ok()).toBe(true);
});
