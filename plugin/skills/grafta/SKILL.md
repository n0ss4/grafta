---
name: grafta
description: Read, change or create Figma .fig files with grafta's MCP tools, and the file open in Figma's desktop app with grafta's panel. Use when the person names a .fig file, speaks of their selection or of what they have open in Figma, asks to inspect, edit, redesign or build a Figma design, or to write a design system's tokens, styles or components into a file.
---

# Working on a .fig with grafta

grafta reads and writes Figma's `.fig` format exactly, offline. It does not
interpret a design: you do. It answers what is in the file, and writes what you
decide. A person then opens the saved file in Figma.

A `.fig` (Figma Design) or a `.deck` (Figma Slides) comes from Figma's
**File → Save local copy…**. In a deck, slides are `SLIDE` nodes in a
`SLIDE_ROW`: duplicate, move or delete them there, and grafta keeps the deck's
"next slide" links in order. A `.jam` (FigJam) has stickies, shapes with text,
connectors and tables, read in the plugin API's names (`text.characters`,
`fills`, `connectorStart`, `cells`); set a sticky's or shape's `text` and `fills`
with `set_properties`, and FigJam lays it out again when the file opens. grafta never needs a
Figma account or the network.

## The loop

1. **Open.** `open_file { path }` returns a `handle`. Every other call takes it.
   Opening a large file takes seconds; every call after answers in
   milliseconds, so keep it open while you work.
2. **Find your way.** `summarize` for pages and counts, then `search_nodes` by
   name or type. Do not walk the tree node by node.
3. **Read.** `read_node` returns a node in the Figma plugin API's names,
   leaving out values at Figma's defaults. `list_components`,
   `list_variables` and `list_styles` give the file's design system: reuse it
   before inventing values.
4. **Write.** Build with `create_frame`, `create_text`, `create_shape`,
   `create_vector`, `create_instance`; change with `set_properties`,
   `override_instance`, `move_node`. Make what repeats a component
   (`create_component`, `create_variant_set`, `add_component_property`) and
   colours, spacing and radii variables (`create_variable_collection`,
   `create_variable`), then bind to them. Many copies from data (a list, cards,
   localised screens) are one `fill_template` call, from rows or a CSV.
5. **Look.** `render_node` draws a frame as a picture, unsaved edits included.
   Check every screen before you save. Its caption names what it drew
   approximately: say so to the person rather than guessing past it.
   Before saving a screen, `check_contrast` on it lists the texts that fail
   WCAG AA in the mode it shows; check each mode a frame is shown in.
6. **Save.** `save_as { path }` to a **new** path. It never overwrites, and
   re-reads the file before it reports. Save checkpoints on long work.
7. **Hand over.** Tell the person the path, to open it in Figma. For review
   or handoff, `export_pdf` writes chosen frames to one PDF, a page each.

Starting from nothing: `new_file { path, from }` takes only the format of any
`.fig` the person has, and none of its content.

From HTML: take a snapshot of the page with `scripts/snapshot.js` (in your
browser tool, `graftaSnapshot('<selector>')`, saved as JSON), then
`import_layout`. Read its `not_imported` list to the person.

## The file open in Figma

When the person speaks of **their selection** or of **what they have open**,
call `list_open` first. A file open in Figma's desktop app with grafta's panel
on it is listed there with `in: "Figma"`, a `handle`, the page they are on and
what they have selected. Work on that handle as on a file, with these
differences:

- Each change shows in Figma at once, and Figma saves it: there is no
  `save_as`, and no `unsaved_edits`. One call is one step of Figma's undo.
- `open_file`, `close_file`, `new_file`, `save_as` and `read_raw` are not
  there, and a few others are not built there yet (`export_pdf`,
  `check_contrast`, `export_tokens`, `fill_template`, `import_layout`): each is
  refused, saying why, with nothing changed. To use them, the person saves a
  local copy (**File → Save local copy…**) and you open that.
- `render_node` is drawn by Figma itself. `summarize`, `search_nodes` and
  `list_components` go a page at a time within a time budget and say how far
  they went.
- The person sees each call in the panel and can press **Stop**: then every
  call is refused until they press Resume. Say so rather than retrying.
- What Figma refuses, a plan's limit included (a second variable mode on a
  plan without modes), comes back in Figma's words. Tell the person; do not
  work around it.

No file listed with `in: "Figma"` means no panel is connected: ask the person
to open grafta's panel on the file in Figma and allow their AI app when it
asks.

## Habits that matter

- **Never overwrite the person's file.** Always `save_as` to a new name.
- **Ask for less.** Results are bounded; when something is left out the result
  says so under `omitted` and how to get it. Follow that, rather than asking
  for a whole page.
- **Auto-layout over absolute positions** for anything that stacks or wraps:
  `layoutMode`, `itemSpacing`, padding, and `layoutSizingHorizontal` /
  `Vertical` (`FIXED`, `HUG`, `FILL`).
- **Icons are vectors.** Draw them from SVG path data (Lucide, Heroicons) with
  `create_vector`, not emoji or text symbols a font may not have.
- **Dark mode is a variable mode.** Give colours a collection with Light and
  Dark modes, bind fills to them, and set a frame's `explicitVariableModes`.
  `render_node` draws the frame in its mode.
- **Fonts are the machine's.** A picture uses the fonts installed where grafta
  runs. If its caption says a font was missing, the fix is installing that font
  and restarting the server, not changing the design.
- **Close large files** with `close_file` when done: an open file holds memory
  which grows with its size (1.3 GB for a 57 MB file, 5.5 GB for an 86 MB one, as measured).
- **Unsaved edits survive a restart.** If the server restarts, `open_file` the
  same path again and the result says how many edits it `recovered`.

## What grafta will refuse

It refuses, and says why, rather than writing something Figma would repair:
deleting a node something else points into, changing variables or styles it did
not create, overwriting a file. Read the refusal and adjust.
