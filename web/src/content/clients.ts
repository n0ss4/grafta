import type { Client } from "./types";

const json = '{ "mcpServers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }';
const then = "Then: claude mcp add grafta -- grafta mcp";

// As README.md's Install section has them.
export const clients: Client[] = [
  {
    id: "claude-code",
    name: "Claude Code",
    kind: "command",
    snippet: "/plugin install grafta --marketplace n0ss4/grafta",
    note: "In Claude Code. Registers the server and teaches the model to use it.",
  },
  {
    id: "codex",
    name: "Codex",
    kind: "command",
    snippet: "codex mcp add grafta -- npx -y grafta mcp",
    note: "In a terminal. Codex's CLI and IDE extension share the server (developers.openai.com/codex/mcp).",
  },
  {
    id: "claude-desktop",
    name: "Claude Desktop",
    kind: "download",
    snippet: "https://github.com/n0ss4/grafta/releases/latest/download/grafta.mcpb",
    note: "Download it and open it: Claude Desktop installs it from Settings → Extensions. No terminal.",
  },
  {
    id: "cursor",
    name: "Cursor",
    kind: "json",
    snippet: json,
    file: ".cursor/mcp.json (or ~/.cursor/mcp.json)",
    note: "Cursor starts the server with npx; nothing else to install.",
  },
  {
    id: "vscode",
    name: "VS Code",
    kind: "json",
    snippet: '{ "servers": { "grafta": { "command": "npx", "args": ["-y", "grafta", "mcp"] } } }',
    file: ".vscode/mcp.json",
    note: "In your workspace. VS Code starts the server with npx.",
  },
  {
    id: "windsurf",
    name: "Windsurf",
    kind: "json",
    snippet: json,
    file: "~/.codeium/windsurf/mcp_config.json",
    note: "The same configuration as Cursor.",
  },
  {
    id: "unix",
    name: "macOS · Linux",
    kind: "command",
    snippet: "curl -fsSL https://raw.githubusercontent.com/n0ss4/grafta/main/install.sh | sh",
    note: `One self-contained binary. ${then}`,
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
    note: `PowerShell. ${then}`,
  },
  {
    id: "rust",
    name: "Rust",
    kind: "command",
    snippet: "cargo install grafta",
    note: "From source, with a Rust toolchain.",
  },
];
