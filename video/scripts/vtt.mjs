// Writes the scenes as WebVTT for the website, from video/src/scenes.ts.
import { build } from "esbuild";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const bundle = join(here, "out", "scenes.mjs");
mkdirSync(dirname(bundle), { recursive: true });
await build({ entryPoints: [join(here, "src", "scenes.ts")], bundle: true, platform: "node", format: "esm", outfile: bundle, logLevel: "error" });
const { vtt } = await import(pathToFileURL(bundle).href);
const out = process.argv[2] ?? join(here, "..", "web", "public", "showcase", "video", "demo.vtt");
writeFileSync(out, vtt());
console.log(`vtt: ${out}`);
