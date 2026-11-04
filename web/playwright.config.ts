import { defineConfig, devices } from "@playwright/test";

// Against the built site, as GitHub Pages serves it under /grafta/.
export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  reporter: process.env.CI ? "github" : "list",
  use: { baseURL: "http://localhost:4173/grafta/" },
  webServer: {
    command: "npm run preview",
    url: "http://localhost:4173/grafta/",
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] }, testIgnore: /no-script/ },
    { name: "phone", use: { ...devices["Pixel 7"] }, testIgnore: /no-script|a11y/ },
    {
      name: "no-script",
      use: { ...devices["Desktop Chrome"], javaScriptEnabled: false },
      testMatch: /no-script/,
    },
  ],
});
