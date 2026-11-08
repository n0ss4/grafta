// Gathers what the scene draws into public/ (not committed): the session's
// pictures from the website, the logos in assets/, and, when Claude Desktop is
// installed, its fonts, so its window is set in them. The fonts are Anthropic's:
// they are read from the app on the machine that renders, and end up only as
// pixels in the video, never in this repository or on the website.
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const web = join(here, "..", "web", "public");
const out = join(here, "public");
rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, "fonts"), { recursive: true });

for (const file of ["showcase/talk/before.png", "showcase/talk/light.png", "showcase/talk/code.png", "brand/logo-light.svg"]) {
  mkdirSync(dirname(join(out, file)), { recursive: true });
  cpSync(join(web, file), join(out, file));
}
cpSync(join(here, "assets", "logos"), join(out, "logos"), { recursive: true });

const claudeFonts = "/Applications/Claude.app/Contents/Resources/fonts";
for (const [from, to] of [
  ["AnthropicSans-Romans-Variable-25x258.ttf", "sans.ttf"],
  ["AnthropicSerif-Romans-Variable-25x258.ttf", "serif.ttf"],
]) {
  if (existsSync(join(claudeFonts, from))) cpSync(join(claudeFonts, from), join(out, "fonts", to));
  else console.warn(`prepare: ${from} not found, Claude's window falls back to Inter`);
}
