// Lay build/client out as GitHub Pages serves it under /grafta/: the router
// writes pre-rendered pages under its basename (build/client/grafta/), so they
// move up beside the assets; the pre-rendered /404 becomes 404.html; and the
// sitemap lists the pages (react-router.config.ts).
import { cpSync, existsSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const web = join(dirname(fileURLToPath(import.meta.url)), "..");
const client = join(web, "build/client");
const nested = join(client, "grafta");

rmSync(join(client, "index.html"), { force: true }); // the SPA fallback: every page is pre-rendered
cpSync(nested, client, { recursive: true });
rmSync(nested, { recursive: true });
renameSync(join(client, "404/index.html"), join(client, "404.html"));
rmSync(join(client, "404"), { recursive: true });
rmSync(join(client, "404.data"), { force: true });

const config = readFileSync(join(web, "react-router.config.ts"), "utf-8");
const pages = JSON.parse(/export const pages = (\[[^\]]*\])/.exec(config)?.[1] ?? "[]");
const site = "https://n0ss4.github.io/grafta";
writeFileSync(
  join(client, "sitemap.xml"),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages
    .map((page) => `  <url><loc>${site}${page === "/" ? "/" : `${page}/`}</loc></url>`)
    .join("\n")}\n</urlset>\n`,
);

for (const page of [
  "index.html",
  "tools/index.html",
  "changelog/index.html",
  "404.html",
  "tools.html",
  "robots.txt",
]) {
  if (!existsSync(join(client, page))) throw new Error(`build/client/${page} is missing`);
}
