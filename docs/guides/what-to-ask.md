# What to ask

Prompts that work as written. The model chooses the tools.

| Job | Ask | Tools it reaches for |
|---|---|---|
| **Understand it** | *Open handoff.fig with grafta. What's in it, how is it organised, and which colours and components does it use?* | `summarize`, `search_nodes`, `read_node` |
| **Turn it into code** | *Open checkout.fig with grafta. Build the payment screen in our stack, with the file's exact sizes, colours and icons.* | `read_node`, `list_variables`, `render_node` |
| **A json-render spec** | *Using grafta, open shadcn.fig and write the dialog as a json-render spec for @json-render/shadcn's catalog.* ([guide](json-render.md)) | `search_nodes`, `read_node`, `render_node` |
| **Dark mode** | *Open app.fig with grafta. Turn the colours on the Settings screen into Light and Dark variables, bind every fill to them, render both modes, and save as app-dark.fig.* | `create_variable` `set_properties` `render_node` |
| **Design-system audit** | *Open library.fig with grafta. List its components, variables and styles, and tell me which colours are used on screens without being a variable or a style.* | `list_components` `list_variables` `search_nodes` |
| **Design tokens** | *Open library.fig with grafta and export its variables as design tokens into tokens/.* ([W3C Design Tokens](https://www.designtokens.org/), a file per mode, as Figma imports them) | `export_tokens` |
| **Accessibility** | *Open checkout.fig with grafta and check every screen's text contrast against WCAG AA. Fix what fails with the file's own colours, and show me before and after.* | `check_contrast` `set_properties` `render_node` |
| **Screens from data** | *Open shop.fig with grafta. Place the Product card component once per row of products.csv in a wrapping grid, with each row's name, price and photo.* | `fill_template` |
| **HTML to layers** | *Import page.json, a snapshot of our landing page, into a new page of site.fig with grafta.* | `import_layout` |
| **A deck** | *Open pitch.deck with grafta. Add three slides after the title with our Q3 numbers, a fade between them, and speaker notes; then export every slide as one PDF.* | `duplicate_node` `set_properties` `export_pdf` |

In CI, without a model: `grafta tool --batch` with `open_file` then `export_tokens` writes the same files from a `.fig` in the repository.

## What it can do

| Job | Tools | |
|---|---|---|
| **Find your way** | `open_file` `summarize` `list_screens` `search_nodes` `list_open` `close_file` | pages, a page's screens (a deck's slides, a site's breakpoints), counts, nodes by name or type, with their path |
| **Read** | `read_node` `read_raw` `list_components` `component_instances` `get_image` | a node exactly, in the Figma plugin API's names |
| **See** | `render_node` `export_pdf` `check_contrast` | a frame as a PNG, unsaved edits included, in its variable mode; frames as a vector PDF, a page each; every text's WCAG AA contrast against what is drawn behind it |
| **Draw** | `create_page` `create_frame` `create_text` `create_shape` `create_vector` `create_instance` `create_section` | frames, rich text, shapes, SVG icons, instances |
| **Import** | `import_layout` | a web page's layout as frames, text, images and auto-layout, from a snapshot a browser takes ([guide](from-a-web-page.md)) |
| **Change** | `set_properties` `override_instance` `move_node` `duplicate_node` `group_nodes` `ungroup` `delete_node` | fills, strokes, effects, auto-layout, sizing, constraints, images, order |
| **Design system** | `create_component` `create_variant_set` `set_variant` `add_component_property` `fill_template` | components, variants and their properties; a component placed once per row of JSON or CSV |
| **Tokens** | `list_variables` `export_tokens` `create_variable_collection` `create_variable` `set_variable` `set_variable_collection` | collections with modes (Light, Dark), aliases, bindings; exported as W3C Design Tokens, a file per mode, as Figma imports them |
| **Styles** | `list_styles` `create_style` `set_style` | colour, text and effect styles |
| **Save** | `new_file` `save_as` | a new file from nothing, or the open one with its edits |

Every tool, with its arguments, is in the [tool reference](../TOOLS.md),
generated from the server's own tool list
([on the website](https://n0ss4.github.io/grafta/tools/)); `grafta tool`
lists them in a terminal.

