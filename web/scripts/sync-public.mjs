// Copy what the site serves from elsewhere in the repository into public/,
// at the paths the old site published them.
// The copies are git-ignored: each file has one source.
import { cpSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const web = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = join(web, "..");
const pub = join(web, "public");

const copy = (from, to) => {
  mkdirSync(dirname(join(pub, to)), { recursive: true });
  cpSync(join(root, from), join(pub, to));
};

for (const name of readdirSync(join(root, "docs/assets"))) copy(`docs/assets/${name}`, `assets/${name}`);
for (const name of readdirSync(join(root, "docs/brand"))) {
  if (name.endsWith(".svg") || name === "apple-touch-icon.png") copy(`docs/brand/${name}`, `brand/${name}`);
}
for (const name of ["install.sh", "install.ps1"]) copy(name, name);
for (const name of ["snapshot.js", "snapshot.sh"]) copy(`scripts/${name}`, name);
