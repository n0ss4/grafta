# grafta's identity

## The mark

<img src="mark.svg" width="64" alt="">

A **graph**: two nodes joined, and a third joined onto them in the accent. grafta
reads a file as a graph of nodes and grafts a model's work onto it; the third
node is what is grafted on, and what makes the graft take is reading the file
exactly. No background, as Figma's own mark has none: on light the edges and
nodes are Ink and the grafted node Sap; on dark, Paper and Bud. Chosen by the
maintainer on 2026-10-30 after four rounds of candidates, replacing the
three-node stem in a tile, which blurred at favicon size.

| File | Use |
|---|---|
| [`mark.svg`](mark.svg) | the favicon and anywhere the reader's theme decides: Ink and Sap, Paper and Bud in dark mode |
| [`logo.svg`](logo.svg) | the wordmark in Sap, for any background: where the reader's theme is unknown, as in the README (GitHub's theme need not be the system's, and some viewers ignore `<picture>`) |
| [`logo-light.svg`](logo-light.svg) | the mark and wordmark on light backgrounds |
| [`logo-dark.svg`](logo-dark.svg) | the same on dark backgrounds |

`icon-512.png`, the Claude Desktop bundle's icon, is the mark in Sap alone on
transparency (it is shown on light and dark); `apple-touch-icon.png` sits on
Paper, as iOS fills transparency with black.

Regenerate them with `python3 scripts/brand.py` (needs fontTools and Inter
Display). The wordmark is outlined, so it needs no font to render.

Leave clear space of a quarter of the mark's width around it. Do not recolour
the grafted node apart from its accent, rotate or mirror the mark, or set the wordmark
in another font.

## Palette

<img src="palette.svg" alt="Ink, Paper, Sap, Deep Sap, Bud, Rust">

| Name | Hex | Use |
|---|---|---|
| Ink | `#15171C` | text, dark backgrounds |
| Paper | `#F6F4EE` | light backgrounds, text on dark |
| Sap | `#1F9D6B` | the brand colour: the mark's tile, graphics, display type of 24 px and over |
| Deep Sap | `#13704B` | text, links, and buttons with white text: 6.09:1 on white, 5.54:1 on Paper, WCAG AA |
| Bud | `#C6F26B` | the mark's grafted node; highlights on dark |
| Rust | `#E5603A` | limits, "approximate", warnings; never decoration |

Diagrams use Ink and Paper for structure, Sap for grafta, Bud for what a model
adds, and Rust only for what is approximate or refused.

**Sap is not a text colour.** It gives 3.45:1 against white and 3.13:1 against
Paper, under the 4.5:1 WCAG AA asks of text at normal size; `check_contrast`
found it failing in the showcase. Text in Sap, or on Sap, uses Deep Sap
instead; Sap stays for the mark, graphics and display type, where 3:1 is
enough. On dark backgrounds Bud is the accent (13.33:1 on Ink's dark).

## Type

- **Inter Display** SemiBold for the wordmark and headings; **Inter** for prose.
  It is the face of Figma's own interface and of most files grafta reads, so the
  showcase pictures and the site agree.
- **JetBrains Mono** for code and tool names.
- On the web, system fallbacks follow each (`system-ui`, `ui-monospace`).
