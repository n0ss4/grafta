import { expect, test } from "@playwright/test";

// internal:specs/109: a Figma file opened in the page, nothing sent.
const FILE_INPUT = 'input[type="file"][accept=".fig,.deck,.jam,.site"]';

test("the sample opens: its page, its screen drawn, and nothing leaves the browser", async ({ page }) => {
  const sent: string[] = [];
  page.on("request", (request) => {
    if (request.method() !== "GET" || (request.postData() ?? "").length > 0) sent.push(request.url());
  });
  await page.goto("open/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Open a Figma file, without Figma.");
  await page.getByRole("button", { name: /Open the sample/ }).click();

  await expect(page.getByRole("heading", { name: "order.fig", exact: true })).toBeVisible();
  await expect(page.getByText("Figma Design, 23 layers.")).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Pages" }).getByRole("button")).toHaveText(["Showcase"]);
  await expect(page.getByText("One page is Figma's own and not shown.")).toBeVisible();
  await expect(page.locator("li img")).toHaveCount(1);

  await page.getByRole("button", { name: /^Order,/ }).click();
  await expect(page.getByRole("img", { name: /drawn by grafta/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Drawn approximately" })).toBeVisible();
  expect(sent, "only GETs, none carrying the file").toEqual([]);
});

test("a screen downloads as its PNG at the scale chosen", async ({ page }) => {
  await page.goto("open/");
  await page.getByRole("button", { name: /Open the sample/ }).click();
  await page.getByRole("button", { name: /^Order,/ }).click();
  await page.getByText("3×", { exact: true }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download PNG at 3×" }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("Order@3x.png");
  const bytes = await (await file.createReadStream()).toArray();
  expect(Buffer.concat(bytes).subarray(0, 8)).toEqual(
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  );
});

test("the tokens show by mode and download as W3C Design Tokens, a file per mode", async ({ page }) => {
  await page.goto("open/");
  await page.getByRole("button", { name: /Open the sample/ }).click();
  await page.getByRole("tab", { name: "Tokens (9)" }).click();
  const table = page.getByRole("table");
  await expect(table.getByRole("columnheader")).toHaveText(["Variable", "Light", "Dark"]);
  await expect(table.getByRole("row", { name: /^Surface/ })).toContainText("#FFFFFF");
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download 2 token files" }).click();
  expect((await download).suggestedFilename()).toBe("order – tokens.zip");
});

test("what is not a Figma file is refused in grafta's words, naming it", async ({ page }) => {
  await page.goto("open/");
  await page.getByRole("button", { name: /Open the sample/ }).waitFor();
  await page.setInputFiles(FILE_INPUT, {
    name: "notes.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("notes"),
  });
  await expect(page.getByRole("alert")).toContainText("grafta could not open notes.txt: notes.txt is not");
  // And the page still opens a file after.
  await page.getByRole("button", { name: /Open the sample/ }).click();
  await expect(page.getByRole("heading", { name: "order.fig", exact: true })).toBeVisible();
});

test("an empty file is refused before reading", async ({ page }) => {
  await page.goto("open/");
  await page.getByRole("button", { name: /Open the sample/ }).waitFor();
  await page.setInputFiles(FILE_INPUT, {
    name: "empty.fig",
    mimeType: "application/octet-stream",
    buffer: Buffer.alloc(0),
  });
  await expect(page.getByRole("alert")).toHaveText("empty.fig is empty.");
});

test("the nav leads to the page", async ({ page, isMobile }) => {
  test.skip(isMobile, "the nav names the page from a wide screen");
  await page.goto("./");
  await page.getByRole("navigation", { name: "Site" }).getByRole("link", { name: "Open a file" }).click();
  await expect(page).toHaveURL(/\/open\/$/);
});
