// Builds what Figma loads, into dist/: the plugin's code, bundled to one
// file, and the panel, its script inlined into one HTML file (a plugin's panel
// is given as a single document).
import { build } from "esbuild";
import { mkdir, readFile, writeFile } from "node:fs/promises";

// Figma runs the main thread in a sandbox of its own: older syntax there.
const main = "es2017";
const panel = "es2020";
await mkdir("dist", { recursive: true });
await build({ entryPoints: ["src/main.ts"], bundle: true, outfile: "dist/code.js", target: main, logLevel: "warning" });
const ui = await build({ entryPoints: ["src/ui.ts"], bundle: true, write: false, target: panel, format: "iife", logLevel: "warning" });
const html = await readFile("src/ui.html", "utf8");
const script = ui.outputFiles[0].text.replaceAll("</script", "<\\/script");
await writeFile("dist/ui.html", html.replace("<!-- script -->", () => `<script>\n${script}</script>`));

// Every tool grafta says acts on the open file has its function here, and no
// function is for a tool grafta says does not: checked by loading the
// registry, which touches Figma only when a tool runs.
const check = await build({ entryPoints: ["src/tools/index.ts"], bundle: true, write: false, format: "esm", platform: "neutral", logLevel: "warning" });
const { REGISTRY } = await import(`data:text/javascript;base64,${Buffer.from(check.outputFiles[0].text).toString("base64")}`);
const contract = JSON.parse(await readFile("src/contract.json", "utf8"));
const acting = contract.tools.filter((tool) => tool.live !== "cannot").map((tool) => tool.name);
const missing = acting.filter((name) => !REGISTRY[name]);
const extra = Object.keys(REGISTRY).filter((name) => !acting.includes(name));
if (missing.length || extra.length) {
  console.error(`the panel and contract.json disagree. Without a function: ${missing.join(", ") || "none"}. A function grafta refuses: ${extra.join(", ") || "none"}`);
  process.exit(1);
}
