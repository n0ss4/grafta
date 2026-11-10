# Changelog

What changed for someone using grafta, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

## [0.4.0](https://github.com/n0ss4/grafta/compare/v0.3.0...v0.4.0) (2026-11-10)


### Added

* read a prototype: reactions, flows, scrolling and overlays ([#63](https://github.com/n0ss4/grafta/issues/63)) ([2f65551](https://github.com/n0ss4/grafta/commit/2f65551b3e2092baa2d2429a3100975382f2d0cc))
* write a prototype: reactions, flows, scrolling and overlays ([#64](https://github.com/n0ss4/grafta/issues/64)) ([7b95bfc](https://github.com/n0ss4/grafta/commit/7b95bfc641e53a1897fa230fe06c74f8dda3c935))


### Fixed

* a duplicated layer's interactions are its own ([#65](https://github.com/n0ss4/grafta/issues/65)) ([2e41735](https://github.com/n0ss4/grafta/commit/2e41735d53b12b83dea2692c48a5159d3cd6a222))

## [0.3.0](https://github.com/n0ss4/grafta/compare/v0.2.3...v0.3.0) (2026-11-07)


### Added

* **cli:** make grafta tool readable in a terminal ([#35](https://github.com/n0ss4/grafta/issues/35)) ([6b2587d](https://github.com/n0ss4/grafta/commit/6b2587dd41eef21714a78796398eccfb4a69d45b))
* **release:** publish to the official MCP registry ([#32](https://github.com/n0ss4/grafta/issues/32)) ([504063d](https://github.com/n0ss4/grafta/commit/504063d42e9c35f38909a0f445c5e657783ffb65))


### Fixed

* **read:** give a vector's paths when only its network is stored ([#30](https://github.com/n0ss4/grafta/issues/30)) ([5a91904](https://github.com/n0ss4/grafta/commit/5a919047edaff9460e6a120feff83cdd731d008f))
* **render:** draw the linear blend modes as the nearest the renderer has ([#28](https://github.com/n0ss4/grafta/issues/28)) ([4ef0976](https://github.com/n0ss4/grafta/commit/4ef097619f78a3903fa4452ce31f3002f2c590b4))

## [0.2.3](https://github.com/n0ss4/grafta/compare/v0.2.2...v0.2.3) (2026-11-06)


### Fixed

* **order:** keep order keys short, and give each child its own ([#25](https://github.com/n0ss4/grafta/issues/25)) ([bc0d84e](https://github.com/n0ss4/grafta/commit/bc0d84e07248c0e75891a0157e233212a64bdf70))

## [0.2.2](https://github.com/n0ss4/grafta/compare/v0.2.1...v0.2.2) (2026-11-06)


### Fixed

* **render:** draw alone a character its font lacks, and name it ([#20](https://github.com/n0ss4/grafta/issues/20)) ([f8d2df7](https://github.com/n0ss4/grafta/commit/f8d2df7b52be1e7248e3f2ebee3f730bef5a1346))
* **render:** draw the spaces a text run or line starts with ([#19](https://github.com/n0ss4/grafta/issues/19)) ([6626a26](https://github.com/n0ss4/grafta/commit/6626a261738b3100694aa5dc1ed751f104903d32))
* **save:** read and save a file of many repeated layers ([#21](https://github.com/n0ss4/grafta/issues/21)) ([b1f71cb](https://github.com/n0ss4/grafta/commit/b1f71cb5cfc32d452ac17833fd066756d6f3290f))
* **security:** bound a message's values by its packed size ([#22](https://github.com/n0ss4/grafta/issues/22)) ([df609b1](https://github.com/n0ss4/grafta/commit/df609b14e2458f481d43d5722ad40dd21119ce3c))

## [0.2.1](https://github.com/n0ss4/grafta/compare/v0.2.0...v0.2.1) (2026-11-06)


### Fixed

* **shapes:** take an image paint when a shape or vector is made ([#15](https://github.com/n0ss4/grafta/issues/15)) ([e927895](https://github.com/n0ss4/grafta/commit/e92789528badf494226feadf7799f37ef8011be8))


### Changed

* **save:** write the message in zstd, as Figma does ([#16](https://github.com/n0ss4/grafta/issues/16)) ([85db28c](https://github.com/n0ss4/grafta/commit/85db28c069652037cb33f6dcfb3c33a75d22d1ba))

## [0.2.0](https://github.com/n0ss4/grafta/compare/v0.1.1...v0.2.0) (2026-11-06)


### Added

* **plugin:** install in the ChatGPT desktop app and Codex ([#10](https://github.com/n0ss4/grafta/issues/10)) ([b36416c](https://github.com/n0ss4/grafta/commit/b36416c852e42f30b5aaf251507945e19efe96a1))
* **save:** leave out images and blobs nothing names ([#11](https://github.com/n0ss4/grafta/issues/11)) ([323ca47](https://github.com/n0ss4/grafta/commit/323ca47d144414fbbbf629672e679cd89dbdea00))


### Fixed

* **components:** save a layer bound to a property ([#12](https://github.com/n0ss4/grafta/issues/12)) ([3dc4c82](https://github.com/n0ss4/grafta/commit/3dc4c823edb935368735c9c40c2726bae1579341))

## [0.1.1](https://github.com/n0ss4/grafta/compare/v0.1.0...v0.1.1) (2026-11-05)


### Fixed

* **contrast:** measure only the colours a text shows ([bd79780](https://github.com/n0ss4/grafta/commit/bd79780966663d5e246e2f7435c6330ff5ee4bfa))
* **journal:** edits saved elsewhere are not recovered into the source ([bd79780](https://github.com/n0ss4/grafta/commit/bd79780966663d5e246e2f7435c6330ff5ee4bfa))
* **style:** fillStyle null keeps the colours of a text's runs ([bd79780](https://github.com/n0ss4/grafta/commit/bd79780966663d5e246e2f7435c6330ff5ee4bfa))
* **tools:** refuse an argument a tool does not take ([bd79780](https://github.com/n0ss4/grafta/commit/bd79780966663d5e246e2f7435c6330ff5ee4bfa))

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
