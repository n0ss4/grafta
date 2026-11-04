#!/usr/bin/env node
// Runs the grafta binary npm installed for this platform, as one of the
// optional grafta-<os>-<cpu> packages. Nothing is downloaded here: stdio passes
// straight through, which `grafta mcp` needs, and so does the exit code.
"use strict";

const { spawnSync } = require("node:child_process");

const platform = `${process.platform}-${process.arch}`;
const executable = process.platform === "win32" ? "grafta.exe" : "grafta";

let binary;
try {
  binary = require.resolve(`grafta-${platform}/bin/${executable}`);
} catch {
  console.error(
    `grafta: no prebuilt binary for ${platform}.\n` +
      "Install one another way: https://github.com/n0ss4/grafta#install, or: cargo install grafta"
  );
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`grafta: cannot run ${binary}: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
