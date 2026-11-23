---
name: grafta-screens
description: Build screens in a Figma .fig with grafta - from a description, from the person's app code, or from an HTML page through a layout snapshot - with auto-layout, the file's own components and tokens, SVG icons and a prototype. Use when the person asks to design, mock up, lay out or import a screen, page, flow, landing page, dashboard or form into a .fig or into the file they have open in Figma.
---

# Building screens with grafta

The `grafta` skill has the general loop and habits; this one is how to build a
screen that a designer can keep working on.

## 1. Before drawing

1. Open the file (`open_file`, or `list_open` for the file open in Figma), or
   start one with `new_file { path, from }`.
2. Read the design system: `list_components`, `list_variables`, `list_styles`.
   A screen built from the file's own components and tokens is one a designer
   can maintain; a screen of loose rectangles is not.
3. Look at one existing screen like the one you will build (`list_screens`,
   `render_node`, `read_node` with `depth`): its width, margins, spacing and type
   sizes are the ones to follow.
4. Write the screen's structure down before calling anything: the frames, what
   stacks vertically, what sits in a row, which parts are instances.

If the file has no design system and the screen needs one, the
`grafta-design-system` skill builds it first.

## 2. Build from the outside in

1. **Where**: `create_page` or a `create_section` ("Onboarding", "Checkout") on a
   page.
2. **The screen**: `create_frame` at the device's size, then `set_properties`
   with `layoutMode: "VERTICAL"`, padding, `itemSpacing`, `primaryAxisSizingMode`
   and a fill bound to a surface token.
3. **Regions**: a frame per region (header, content, footer) with its own
   auto-layout. A child fills its parent's width with `layoutAlign: "STRETCH"`,
   and takes the space left along the flow with `layoutGrow: 1`.
4. **Parts**: `create_instance` of the file's components, then
   `override_instance` for labels and icon colours, `set_variant` for state,
   `set_properties { componentProperties }` for TEXT, BOOLEAN and swap values.
5. **Text**: `create_text` with the file's text styles (`set_properties { textStyle }`)
   or rich `runs` for a bold word or a link.
6. **Icons**: `create_vector` from SVG path data (Lucide, Heroicons), never an
   emoji or a symbol from a font.
7. **Repeats** (rows, cards, a list from data): `fill_template` with `rows` or a
   `csv`, into an auto-layout frame, in one call.

Prefer auto-layout over x and y for anything that stacks or wraps. Use absolute
positions only for overlaps (a badge on an avatar: `layoutPositioning: "ABSOLUTE"`).

## 3. From the person's code

Read the component's source and its styles, then build it as above, using the
exact values from the code (sizes, colours, spacing, font) and the file's tokens
where a value matches one. Do not round or "improve" values on the way.

## 4. From an HTML page

1. Take a layout snapshot: `snapshot.sh page.html page.json 'main'` (headless
   Chrome, from https://n0ss4.github.io/grafta/snapshot.sh), or in a browser tool
   run grafta's `snapshot.js` and save `JSON.stringify(graftaSnapshot('main'))`.
2. `import_layout { parent, snapshot: "page.json" }`. Flexbox becomes auto-layout;
   `layout: "absolute"` places each box where the browser put it instead.
3. Read `not_imported` to the person (a gradient, a transform, an inline SVG, a
   remote image), then redraw what matters: `create_vector` for an inline SVG's
   paths, `set_properties` with a gradient fill.

## 5. Prototype

`set_properties { reactions: [{ trigger: { type: "ON_CLICK" }, actions: [{ type: "NODE", destinationId, navigation: "NAVIGATE", transition: { type: "SMART_ANIMATE", duration: 0.3, easing: { type: "EASE_OUT" } } }] }] }`
on the tappable layer; `flowStartingPoint { name }` on the first screen.

## 6. Check and hand over

1. `render_node` every screen you made and look at it. The caption's `notes`
   name what was drawn approximately: tell the person rather than guess.
2. `check_contrast` every screen, in each mode it is shown in.
3. `save_as { path }` to a new path; tell the person to open it in Figma, where
   text is laid out again and sizes grafta gave as provisional are settled.

## On the file open in Figma

Each change shows at once and Figma saves it: no `save_as`. `import_layout` and
`fill_template` are not built there yet; for those, the person saves a local copy
and you work on it. Prototype keys (`reactions`) are not built there yet either.
