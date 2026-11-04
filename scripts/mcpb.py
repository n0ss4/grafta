#!/usr/bin/env python3
"""Build grafta's Claude Desktop bundle, grafta.mcpb.

    python3 scripts/mcpb.py <macos binary> <windows binary> <out.mcpb>

The macOS binary is the universal one the release joins with lipo; the format
varies the command per operating system, not per CPU. The version and the tools
are asked of the binary itself (`--version`, and `tools/list` over `grafta mcp`),
so the manifest cannot drift from what the model is served. The manifest is
then checked and packed with the official CLI, pinned below.

Any binary that runs here can stand in for both, to validate the manifest (CI
does this on Linux): only the one this machine can run is asked anything.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

MCPB = "@anthropic-ai/mcpb@2.1.2"
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def version(binary):
    out = subprocess.run([binary, "--version"], capture_output=True, text=True, check=True).stdout
    match = re.fullmatch(r"grafta (\d+\.\d+\.\d+)\s*", out)
    if not match:
        sys.exit(f"{binary} --version said {out!r}, not `grafta X.Y.Z`")
    return match.group(1)


def tools(binary):
    """The server's own tool list, asked the way a client asks it."""
    messages = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize",
         "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "mcpb.py", "version": "0"}}},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
    ]
    server = subprocess.Popen([binary, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    for message in messages:
        server.stdin.write(json.dumps(message) + "\n")
        server.stdin.flush()
    listed = None
    for line in server.stdout:
        reply = json.loads(line)
        if reply.get("id") == 2:
            listed = reply["result"]["tools"]
            break
    server.stdin.close()
    server.wait(timeout=10)
    if not listed:
        sys.exit("the server listed no tools")
    return [{"name": tool["name"], "description": first_sentence(tool.get("description", ""))} for tool in listed]


def first_sentence(text):
    """A description's first sentence: a listing wants one line. A full stop
    before a space ends it, except in "e.g." and "i.e."; a sentence still
    longer than a line ends where its list begins, at its first colon."""
    text = " ".join(text.split())
    match = re.search(r"(?<!e\.g)(?<!i\.e)\.(?=\s)", text)
    sentence = text[: match.start() + 1] if match else text
    if len(sentence) > 160 and ": " in sentence:
        sentence = sentence[: sentence.index(": ")] + "."
    return sentence


def manifest(version_, tools_):
    return {
        "manifest_version": "0.3",
        "name": "grafta",
        "display_name": "grafta",
        "version": version_,
        "description": "Read and edit .fig files saved from Figma: exactly, offline, with no Figma account.",
        "long_description": (
            "grafta gives Claude tools to open a `.fig`, read every node, colour, variable and component "
            "in it, change it, draw it to check, and save a new file Figma opens. It reads and writes the "
            "file on your computer and makes no network call.\n\n"
            "Get a `.fig` from Figma with *File → Save local copy…*, then ask Claude to open it."
        ),
        "author": {"name": "Nossair Ghazouani", "url": "https://github.com/n0ss4"},
        "homepage": "https://n0ss4.github.io/grafta",
        "documentation": "https://github.com/n0ss4/grafta#readme",
        "support": "https://github.com/n0ss4/grafta/issues",
        "repository": {"type": "git", "url": "https://github.com/n0ss4/grafta"},
        "license": "MIT",
        "keywords": ["figma", "fig", "design", "design system", "tokens", "components"],
        "icon": "icon.png",
        "server": {
            "type": "binary",
            "entry_point": "server/grafta",
            "mcp_config": {
                "command": "${__dirname}/server/grafta",
                "args": ["mcp"],
                "env": {},
                "platform_overrides": {"win32": {"command": "${__dirname}/server/grafta.exe"}},
            },
        },
        "tools": tools_,
        "tools_generated": False,
        "compatibility": {"platforms": ["darwin", "win32"]},
    }


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    macos, windows, out = sys.argv[1:]
    runnable = next((b for b in (macos, windows) if os.access(b, os.X_OK) and probe(b)), None)
    if not runnable:
        sys.exit("neither binary runs on this machine, so its tools cannot be asked")
    version_, tools_ = version(runnable), tools(runnable)

    with tempfile.TemporaryDirectory() as work:
        os.makedirs(os.path.join(work, "server"))
        shutil.copy(macos, os.path.join(work, "server", "grafta"))
        shutil.copy(windows, os.path.join(work, "server", "grafta.exe"))
        os.chmod(os.path.join(work, "server", "grafta"), 0o755)
        shutil.copy(os.path.join(ROOT, "docs", "brand", "icon-512.png"), os.path.join(work, "icon.png"))
        with open(os.path.join(work, "manifest.json"), "w") as f:
            json.dump(manifest(version_, tools_), f, indent=2, ensure_ascii=False)
            f.write("\n")
        npx = ["npx", "-y", MCPB]
        subprocess.run(npx + ["validate", os.path.join(work, "manifest.json")], check=True)
        subprocess.run(npx + ["pack", work, os.path.abspath(out)], check=True)
    print(f"{out}: grafta {version_}, {len(tools_)} tools, {os.path.getsize(out):,} bytes")


def probe(binary):
    try:
        return subprocess.run([binary, "--version"], capture_output=True, timeout=10).returncode == 0
    except OSError:
        return False


if __name__ == "__main__":
    main()
