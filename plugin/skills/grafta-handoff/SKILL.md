---
name: grafta-handoff
description: Hand a Figma .fig over to development with grafta - design tokens exported as W3C Design Tokens JSON, screens as one PDF, pictures of each screen, and a spec per screen with exact sizes, spacing, tokens, text styles, components and interactions read from the file. Use when the person asks for a handoff, dev specs, redlines, exported tokens, a PDF for review, or "what does a developer need" from a .fig.
---

# Handoff with grafta

A handoff gives a developer what the file says, exactly, and names what it does
not say. Every value in it comes from a grafta result; nothing is estimated,
rounded or filled in.

## 1. Agree the scope

`open_file`, `summarize`, `list_screens { page }`. Ask which screens or flows are
being handed over, and where the outputs go (a `handoff/` folder in their repo,
for instance). Make that folder before exporting: grafta writes only new files
into an existing folder.

## 2. Tokens

`export_tokens { dir }` writes one file per collection and mode,
`<collection>.<mode>.tokens.json`, in W3C Design Tokens format, the way Figma
imports them. Pass `collection` for one collection only. Read the result to the
person:

- `unresolved`: what was not written, and why.
- `renamed`: every name changed to fit the format, old and new.
- `notes`.

It never writes over a file: to export again, use a new or empty folder.

## 3. Screens as a PDF

`export_pdf { ids, path }` writes the chosen frames to one new PDF, a page each in
the order given, as vectors, with the text grafta wrote kept selectable. Read the
result's per-page `notes` to the person, as for a picture: a shadow or blur turns
what it covers into an image.

## 4. Pictures

`render_node { id, scale: 2 }` per screen. To write PNG files for a repository or
a ticket, use a batch (the `grafta-batch` skill) with `"out": "handoff/<screen>.png"`
on each `render_node` call.

## 5. A spec per screen

`read_node { id, depth }` on the screen and its regions; follow `omitted` and
`children_from` where the result is cut. For each screen, write:

| Section | From |
|---|---|
| Size and layout | the frame's `width`, `height`, `layoutMode`, padding, `itemSpacing`, alignment and sizing modes |
| Tokens | each bound `variable` on fills, strokes, spacing, radii and sizes, by name; an unbound value as its hex or number, marked unbound |
| Type | each text's `textStyle`, or its font, size, line height and letter spacing when it has none |
| Components | each instance's component and variant values (`read_node` gives them), and its overrides |
| States | the variant set's properties and values (`list_components`) |
| Interactions | each layer's `reactions`: trigger, destination, transition |
| Assets | icons as vectors (`read_node { geometry: true }` gives the SVG path data); images by `imageHash`, fetched with `get_image` |

Write the spec as Markdown in the folder agreed in step 1, one file per screen or
one with a section each. Use the file's names for layers and tokens so a developer
can find them in Figma.

**Name what the file does not say.** Responsive behaviour beyond constraints and
auto-layout, hover and focus states with no variant, motion with no reaction:
write "not in the file" for each, rather than proposing values. Proposals, if the
person wants them, go in a separate section marked as yours.

## 6. In CI, without a model

The same exports run from a terminal with a batch:

```json
[
  { "tool": "open_file", "args": { "path": "design/app.fig" } },
  { "tool": "export_tokens", "args": { "handle": "$0.handle", "dir": "tokens" } }
]
```

`grafta tool --batch handoff.json`. The `grafta-batch` skill covers the format.

## On the file open in Figma

`export_tokens` and `export_pdf` are not built there yet. `read_node`,
`list_components`, `list_variables` and `render_node` work there. For the
exports, ask the person to save a local copy (**File → Save local copy…**) and
open that.
