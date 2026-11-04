#!/bin/sh
# One version for every channel. The workspace's Cargo.toml is the
# source; the npm packages, the Claude Code plugin and the marketplace follow.
#
#   scripts/set-version.sh 0.2.0        write 0.2.0 everywhere
#   scripts/set-version.sh --check      fail unless every file names one version
#   scripts/set-version.sh --check 0.2.0  ... and that version is 0.2.0
set -eu
cd "$(dirname "$0")/.."

python3 - "$@" <<'PY'
import json, re, sys

FILES = {
    "Cargo.toml": [
        r'(\[workspace\.package\]\nversion = ")([^"]+)(")',
        r'(package = "grafta-[a-z]+", version = ")([^"]+)(")',
    ],
    "npm/grafta/package.json": [r'(\n  "version": ")([^"]+)(")', r'("grafta-[a-z0-9-]+": ")([^"]+)(")'],
    "plugin/.claude-plugin/plugin.json": [r'("version": ")([^"]+)(")'],
    ".claude-plugin/marketplace.json": [r'("version": ")([^"]+)(")'],
    "plugin/.mcp.json": [r'("grafta@)([^"]+)(")'],
}

args = sys.argv[1:]
check = bool(args) and args[0] == "--check"
wanted = (args[1:] if check else args)[:1]
if not check and not wanted:
    sys.exit("usage: set-version.sh <version> | --check [version]")
if wanted and not re.fullmatch(r"\d+\.\d+\.\d+", wanted[0]):
    sys.exit(f"not a version: {wanted[0]}")

found = {}
for path, patterns in FILES.items():
    text = open(path).read()
    for pattern in patterns:
        hits = re.findall(pattern, text)
        if not hits:
            sys.exit(f"{path}: no version where {pattern!r} expects one")
        for hit in hits:
            found.setdefault(hit[1], []).append(path)
        if not check:
            text = re.sub(pattern, lambda m: m.group(1) + wanted[0] + m.group(3), text)
    if not check:
        open(path, "w").write(text)

if check:
    if len(found) != 1:
        sys.exit("versions disagree: " + "; ".join(f"{v} in {sorted(set(p))}" for v, p in found.items()))
    (version,) = found
    if wanted and version != wanted[0]:
        sys.exit(f"the files say {version}, the tag says {wanted[0]}")
    print(f"every channel names {version}")
else:
    print(f"set {wanted[0]} in {len(FILES)} files; run cargo update -w to refresh Cargo.lock")
PY
