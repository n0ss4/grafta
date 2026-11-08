import { expect, test } from "@playwright/test";

// internal:specs/103: the line, the button and the whole video in the first screen.
const desktops = [
  [1280, 720],
  [1366, 768],
  [1440, 900],
  [1536, 864],
  [1920, 1080],
] as const;
const others = [
  [768, 1024],
  [390, 844],
] as const;

for (const [width, height] of [...desktops, ...others]) {
  test(`at ${width}×${height} the line, the button, the video and its scenes are in the first screen`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height });
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("./");
    await expect(page.getByRole("heading", { level: 1 })).toBeInViewport({ ratio: 1 });
    await expect(page.locator("#hero").getByRole("button", { name: "Get grafta" })).toBeInViewport({
      ratio: 1,
    });
    await expect(page.locator("#hero video")).toBeInViewport({ ratio: 1 });
    await expect(page.getByRole("list", { name: "Scenes of the video" })).toBeInViewport({ ratio: 1 });
  });
}

test("at 1440×900 the video shows the chat's text at least as large as before (SC-002)", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("./");
  // The chat's 21 px text, scaled by the crop (1920/1700), then by the width shown.
  const width = await page.locator("#hero video").evaluate((node) => node.getBoundingClientRect().width);
  expect(((21 * 1920) / 1700) * (width / 1920)).toBeGreaterThanOrEqual(13.8);
});

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

test("picking a scene goes to its start and marks it (SC-003)", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  const video = page.locator("#hero video");
  await expect.poll(() => video.evaluate((node: HTMLVideoElement) => node.readyState)).toBeGreaterThan(0);
  const rail = page.getByRole("list", { name: "Scenes of the video" });
  const buttons = rail.getByRole("button");
  await expect(buttons).toHaveCount(6);
  for (const [name, start] of [
    ["Turn it into code", 24.333],
    ["Ask what's in it", 8.267],
    ["Open it in Figma", 35.933],
  ] as const) {
    await rail.getByRole("button", { name: new RegExp(`^${name}`) }).click();
    await expect
      .poll(() => video.evaluate((node: HTMLVideoElement) => node.currentTime))
      .toBeGreaterThanOrEqual(start - 0.5);
    expect(await video.evaluate((node: HTMLVideoElement) => node.currentTime)).toBeLessThan(start + 0.5);
    await expect(rail.getByRole("button", { name: new RegExp(`^${name}`) })).toHaveAttribute(
      "aria-current",
      "step",
    );
  }
});

test("its files are served", async ({ page, request }) => {
  await page.goto("./");
  const sources = await page
    .locator("#hero video source, #hero video track")
    .evaluateAll((nodes) => nodes.map((node) => (node as HTMLSourceElement).src));
  expect(sources).toHaveLength(3);
  for (const source of sources) expect((await request.get(source)).ok()).toBe(true);
  const poster = await page.locator("#hero video").evaluate((node: HTMLVideoElement) => node.poster);
  expect((await request.get(poster)).ok()).toBe(true);
});
