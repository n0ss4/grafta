# Install

| Where | Run | Needs |
|---|---|---|
| **Claude Desktop** | download [`grafta.mcpb`](https://github.com/n0ss4/grafta/releases/latest/download/grafta.mcpb) and open it | nothing else |
| **Claude Code** | `/plugin install grafta --marketplace n0ss4/grafta` | Node.js 18+ |
| **ChatGPT desktop, Codex** | `codex plugin marketplace add n0ss4/grafta@main && codex plugin add grafta@grafta` | Node.js 18+ |
| **OpenCode** | `opencode mcp add grafta --global -- npx -y grafta mcp` | Node.js 18+ |

Claude Desktop installs the bundle from *Settings → Extensions*. The Claude Code plugin registers the server and teaches the model how to use it. For OpenCode, `--global` registers it for every project; leave it out for the current one only.

## As a program

| Platform | Command |
|---|---|
| macOS, Linux | `curl -fsSL https://raw.githubusercontent.com/n0ss4/grafta/main/install.sh \| sh` |
| Homebrew | `brew install n0ss4/tap/grafta` |
| Windows | `irm https://raw.githubusercontent.com/n0ss4/grafta/main/install.ps1 \| iex` |
| npm | `npx -y grafta mcp` |
| Rust | `cargo install grafta` |

Then register it once, for example in Claude Code:

```bash
claude mcp add grafta -- grafta mcp
```

Or in a client configured with JSON:

| Client | File | Configuration |
|---|---|---|
| **Cursor** | `.cursor/mcp.json` (or `~/.cursor/mcp.json`) | `{ "mcpServers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }` |
| **Claude Desktop**, by hand | `claude_desktop_config.json` | the same as Cursor |
| **VS Code** | `.vscode/mcp.json` | `{ "servers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }` |

With grafta installed by the script, Homebrew or Cargo, `"command": "grafta", "args": ["mcp"]` does the same without npm.

Prebuilt for macOS (Apple silicon and Intel), Linux (x86_64 and arm64) and Windows (x86_64), as one self-contained binary. The binaries are not signed: if macOS says it cannot check grafta the first time it runs, allow it in *System Settings → Privacy & Security*. We have run it by hand on a Mac with Apple silicon, in Claude Code and Claude Desktop; the tests also pass on Linux and Windows in CI. The other apps use the same setup and have not been tried by us. ChatGPT on the web reads no local server, so it cannot use grafta yet.

If something does not work, [Troubleshooting](TROUBLESHOOTING.md) goes by what you see.


## Try it

Ask in words, in your AI app:

> Using grafta, what have I selected in Figma?

> Using grafta, what's in design.fig?

> Make it light, and check the contrast. Now turn it into code.

A Figma file on your computer comes from *File → Save local copy…* in Figma. In Claude Desktop, say where the file is, like `~/Downloads/design.fig`: a file uploaded to the chat lives in Claude's cloud, so Claude copies it to a folder on your Mac that you allow, and grafta opens it there.

No AI app at hand? [Open a Figma file in your browser](https://n0ss4.github.io/grafta/open/): its pages and screens, PNGs and design tokens, with grafta running in the page and nothing uploaded.

[What to ask](guides/what-to-ask.md) has prompts for code, dark mode, audits, tokens, accessibility, screens from data and decks.
