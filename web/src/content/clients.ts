import type { Client } from "./types";

const json = '{ "mcpServers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }';
const then = "Then, in Claude Code: claude mcp add grafta -- grafta mcp";
const NODE = "Needs Node.js 18 or newer.";

// As README.md's Install section has them: the AI apps, Claude Desktop first (the
// app of the home's video, and the one with no terminal), then the program.
export const clients: Client[] = [
  {
    id: "claude-desktop",
    group: "app",
    name: "Claude Desktop",
    kind: "download",
    snippet: "https://github.com/n0ss4/grafta/releases/latest/download/grafta.mcpb",
    note: "Download it and open it: Claude Desktop installs it from Settings → Extensions. No terminal, no Node.js. The binary is not signed, so your system may ask first.",
  },
  {
    id: "claude-code",
    group: "app",
    name: "Claude Code",
    kind: "command",
    snippet: "/plugin install grafta --marketplace n0ss4/grafta",
    note: `In Claude Code. Registers the server and teaches the model to use it. ${NODE}`,
  },
  {
    id: "chatgpt",
    group: "app",
    name: "ChatGPT desktop",
    kind: "command",
    snippet: "codex plugin marketplace add n0ss4/grafta@main && codex plugin add grafta@grafta",
    note: `In a terminal, then restart the app. Or in the app: Settings → MCP servers → Add server → STDIO, with the command npx -y grafta mcp. Desktop only: ChatGPT on the web reads no local server. ${NODE} Not yet tried by us.`,
  },
  {
    id: "codex",
    group: "app",
    name: "Codex",
    kind: "command",
    snippet: "codex mcp add grafta -- npx -y grafta mcp",
    note: `In a terminal. ${NODE} Set up by Codex's documentation; not yet tried by us.`,
  },

  {
    id: "cursor",
    group: "app",
    name: "Cursor",
    kind: "json",
    snippet: json,
    file: ".cursor/mcp.json (or ~/.cursor/mcp.json)",
    note: `Cursor starts the server with npx. ${NODE} Not yet tried by us.`,
  },
  {
    id: "vscode",
    group: "app",
    name: "VS Code",
    kind: "json",
    snippet: '{ "servers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }',
    file: ".vscode/mcp.json",
    note: `In your workspace. VS Code starts the server with npx. ${NODE} Not yet tried by us.`,
  },
  {
    id: "windsurf",
    group: "app",
    name: "Windsurf",
    kind: "json",
    snippet: json,
    file: "~/.codeium/windsurf/mcp_config.json",
    note: `The same configuration as Cursor. ${NODE} Not yet tried by us.`,
  },
  {
    id: "unix",
    group: "program",
    name: "macOS · Linux",
    kind: "command",
    snippet: "curl -fsSL https://raw.githubusercontent.com/n0ss4/grafta/main/install.sh | sh",
    note: `One self-contained binary, not signed. ${then}`,
  },
  {
    id: "brew",
    group: "program",
    name: "Homebrew",
    kind: "command",
    snippet: "brew install n0ss4/tap/grafta",
    note: then,
  },
  {
    id: "windows",
    group: "program",
    name: "Windows",
    kind: "command",
    snippet: "irm https://raw.githubusercontent.com/n0ss4/grafta/main/install.ps1 | iex",
    note: `PowerShell. Not signed, so SmartScreen may warn. Built and tested in CI, not yet tried by hand. ${then}`,
  },
  {
    id: "rust",
    group: "program",
    name: "Rust",
    kind: "command",
    snippet: "cargo install grafta",
    note: "From source, with a Rust toolchain.",
  },
];

/** The sample file, and the first thing to ask about it (the message's). */
export const sample = { file: "order.fig", href: "sample/order.fig" };
export const firstRequest = "Using grafta, what's in order.fig?";
