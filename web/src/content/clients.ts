import type { Client } from "./types";

const json = '{ "mcpServers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }';
const then = "Then, in Claude Code: claude mcp add grafta -- grafta mcp";
const NODE = "Needs Node.js 18 or newer.";

// As README.md's Install section has them.
export const clients: Client[] = [
  {
    id: "claude-code",
    name: "Claude Code",
    kind: "command",
    snippet: "/plugin install grafta --marketplace n0ss4/grafta",
    note: `In Claude Code. Registers the server and teaches the model to use it. ${NODE}`,
  },
  {
    id: "codex",
    name: "Codex",
    kind: "command",
    snippet: "codex mcp add grafta -- npx -y grafta mcp",
    note: `In a terminal. ${NODE} Set up by Codex's documentation; not yet tried by us.`,
  },
  {
    id: "claude-desktop",
    name: "Claude Desktop",
    kind: "download",
    snippet: "https://github.com/n0ss4/grafta/releases/latest/download/grafta.mcpb",
    note: "Download it and open it: Claude Desktop installs it from Settings → Extensions. No terminal, no Node.js. The binary is not signed, so your system may ask first.",
  },
  {
    id: "cursor",
    name: "Cursor",
    kind: "json",
    snippet: json,
    file: ".cursor/mcp.json (or ~/.cursor/mcp.json)",
    note: `Cursor starts the server with npx. ${NODE} Not yet tried by us.`,
  },
  {
    id: "vscode",
    name: "VS Code",
    kind: "json",
    snippet: '{ "servers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }',
    file: ".vscode/mcp.json",
    note: `In your workspace. VS Code starts the server with npx. ${NODE} Not yet tried by us.`,
  },
  {
    id: "windsurf",
    name: "Windsurf",
    kind: "json",
    snippet: json,
    file: "~/.codeium/windsurf/mcp_config.json",
    note: `The same configuration as Cursor. ${NODE} Not yet tried by us.`,
  },
  {
    id: "unix",
    name: "macOS · Linux",
    kind: "command",
    snippet: "curl -fsSL https://raw.githubusercontent.com/n0ss4/grafta/main/install.sh | sh",
    note: `One self-contained binary, not signed. ${then}`,
  },
  {
    id: "brew",
    name: "Homebrew",
    kind: "command",
    snippet: "brew install n0ss4/tap/grafta",
    note: then,
  },
  {
    id: "windows",
    name: "Windows",
    kind: "command",
    snippet: "irm https://raw.githubusercontent.com/n0ss4/grafta/main/install.ps1 | iex",
    note: `PowerShell. Not signed, so SmartScreen may warn. Built and tested in CI, not yet tried by hand. ${then}`,
  },
  {
    id: "rust",
    name: "Rust",
    kind: "command",
    snippet: "cargo install grafta",
    note: "From source, with a Rust toolchain.",
  },
];
