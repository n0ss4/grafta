/// <reference types="vitest/config" />
import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// The site lives under https://n0ss4.github.io/grafta/. Vitest runs without the
// router's plugin, which takes over the build.
export default defineConfig(({ mode }) => ({
  base: "/grafta/",
  plugins: mode === "test" ? [] : [tailwindcss(), reactRouter()],
  resolve: { tsconfigPaths: true },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
}));
