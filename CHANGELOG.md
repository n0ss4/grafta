# Changelog

What changed for someone using grafta, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-04

The first release.

### Added

- **An MCP server with 43 tools** (`grafta mcp`) and the same tools from a
  terminal (`grafta tool`, `grafta tool --batch`). See the
  [tool reference](docs/TOOLS.md).
- **Install without Rust:** prebuilt binaries for macOS (Apple silicon, Intel),
  Linux (x86_64, arm64) and Windows (x86_64); an install script, Homebrew, npm
  (`npx -y grafta mcp`) and Cargo; a one-click bundle for Claude Desktop
  (`grafta.mcpb`, 043) and a Claude Code plugin that teaches the model the
  workflow.
- **Four products:** Figma Design `.fig`, Figma Slides `.deck`,
  FigJam `.jam` and Figma Sites `.site`, opened, read, drawn
  and saved.
- **Reading:** every node in the plugin API's names, components and their
  instances, variables, styles, embedded images; results bounded, saying what
  they left out.
- **Writing:** frames, rich text, shapes, SVG vectors, instances, sections,
  groups, duplicates; fills, strokes, gradients, effects, auto-layout, sizing,
  constraints and images; components, variant sets and component properties;
  variable collections with modes and aliases, bound to any field; colour,
  text and effect styles, the file's own changed in place; templates filled from JSON or CSV; a web page's
  layout imported as layers; a new file from nothing.
- **Seeing:** `render_node` draws a node offline, in its variable mode, with
  the file's fonts; `export_pdf` writes frames as a vector PDF;
  `check_contrast` checks every text against WCAG AA.
- **Safety:** your files are not written over; unsaved edits are journaled and
  replayed after a restart; a failed call rolls back whole; a
  write Figma would have to repair is refused, with the reason.
- **The website:** a tool reference generated from the server's tool list, a
  social card drawn by grafta, and an FAQ. Its first screen plays real
  sessions recorded in Claude Code, a picture turned into an editable Figma
  file first: a person's request, the model's replies,
  grafta's calls one click away, the file it wrote; and the same request sent
  without grafta beside it.
