---
name: grafta-design-system
description: Write a design system into a Figma .fig with grafta - variable collections with Light and Dark modes and aliases, colour, text and effect styles, components with variants and properties bound to those tokens. Use when the person asks to create, set up or extend design tokens, a theme, dark mode, a component library or a UI kit in a .fig, from code, a brand guide or a description.
---

# A design system in a .fig with grafta

Build in this order. Each step depends on the one before: a component binds to
variables that must already exist, and a variant set groups components that must
already be built.

The `grafta` skill has the general loop (open, read, write, render, save); this one
is what to build and in what order.

## 0. Start and read what exists

- New file: `new_file { path, from }`, where `from` is any `.fig` the person has
  (only its format is used). An existing file: `open_file`, then
  `list_variables`, `list_styles` and `list_components`.
- **Reuse before inventing.** If the file already has tokens or styles, bind to
  them. grafta changes only the variables and collections it made; the file's own
  are read and bound to, and `set_variable` on them is refused. Styles are the
  exception: `set_style` changes the file's own too.
- From code: read the person's tokens (CSS custom properties, Tailwind config, a
  theme file) and carry their names and values over exactly. Ask rather than
  invent a value the source does not have.

## 1. Variables

1. **Primitives**: `create_variable_collection { name: "Primitives", modes: ["Value"] }`,
   then a `create_variable` per raw value: `Blue/500` COLOR, `Space/4` FLOAT,
   `Radius/md` FLOAT. Slashes group them in Figma's panel.
2. **Semantic tokens**: `create_variable_collection { name: "Theme", modes: ["Light", "Dark"] }`,
   then variables whose values alias primitives per mode:
   `{ "Light": { "alias": "Gray/50" }, "Dark": { "alias": "Gray/900" } }`.
   Name them by role: `Surface/Default`, `Text/Primary`, `Border/Subtle`,
   `Action/Primary`.
3. A mode added later: `set_variable_collection { id, add_mode }`; every variable
   takes the first mode's value until you `set_variable` it.

A translucent colour token is a `#RRGGBBAA` value, not a paint opacity: on the
file open in Figma, a bound paint's own opacity is dropped when it is resolved
for another mode.

## 2. Styles

`create_style` for what Figma users expect as styles:

- TEXT: one per step of the type scale (`Heading/L`, `Body/M`, `Label/S`) with
  `fontFamily`, `fontStyle`, `fontSize`, `lineHeight`, `letterSpacing`.
- EFFECT: elevation (`Shadow/Card`, `Shadow/Overlay`).
- FILL: only when the person wants colour styles as well as variables.

## 3. Components

For each component (button, input, card, list row…):

1. `create_frame` on a "Components" page, then `set_properties`: auto-layout
   (`layoutMode`, `itemSpacing`, padding, `primaryAxisSizingMode`), fills bound by
   variable (`fills: [{ type: "SOLID", color: "#…", variable: "Action/Primary" }]`),
   spacing and radii bound (`itemSpacing: { variable: "Space/2" }`,
   `cornerRadius: { variable: "Radius/md" }`), `textStyle` on its texts.
2. Content: `create_text` for labels, `create_vector` for icons from SVG path data
   (Lucide's `viewBox` is `[0, 0, 24, 24]`; strokes, `strokeCap`/`strokeJoin`
   ROUND, no fills).
3. `create_component { id, name: "Button/Primary" }`.
4. Properties: `add_component_property` (TEXT for a label, BOOLEAN to show an
   icon, INSTANCE_SWAP for which icon), then bind the inner layer with
   `set_properties { componentPropertyReferences: { characters: "Label" } }`.

## 4. Variants

Build one component per state or size, side by side in the same parent, then
`create_variant_set { name: "Button", variants: [{ id, values: { "State": "Default", "Size": "M" } }, …] }`.
Every variant names the same properties with a different combination.

Component properties on variants are not written yet: put TEXT, BOOLEAN and
INSTANCE_SWAP properties on standalone components, or tell the person this is a
limit of grafta today.

## 5. Show it and check it

1. A "Foundations" page: swatches (`create_shape` RECTANGLE bound to each colour
   variable, `create_text` with its name), the type scale in its text styles.
2. A preview frame per mode: instances of each component, with
   `explicitVariableModes: { "Theme": "Dark" }` on the dark one.
3. `render_node` each preview and look at it. `check_contrast` each, in each mode.
   Fix tokens, not individual layers, when a pair fails.
4. `save_as { path }` to a new path, and ask the person to open it in Figma.

## On the file open in Figma

Variables, styles, components and variant sets are made there too, and each call
is saved and undoable in Figma. `componentPropertyReferences` is not built there
yet: add the property there, and bind the layer in Figma by hand or work on a
local copy. A plan without modes refuses a second mode in Figma's words: tell the
person, do not work around it.
