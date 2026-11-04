# How grafta is built

One Rust workspace, in layers. A `.fig` is decoded into a typed scene graph; the
domain holds open files as sessions and reaches the world through ports; the
application runs each tool as a typed command or query; products are plugins;
JSON exists only at the edge, where the MCP server and the CLI both call the
same tool list.

```
                 decode                       core                    adapters         front ends
 .fig ─► figfile ─► kiwi ─► schema ─► model ─► domain ─► app ─────────► edge ──┬─► mcp  (grafta mcp)
   ▲                                             ▲  ▲                    │      └─► cli  (grafta tool)
   │                              products ──────┘  └── ports ◄── disk ──┤
   │                    design · slides · figjam · sites       memory    │
   └──────────────────── save_as ◄── unit of work ◄── command ◄──────────┘
                                                 render ◄── render_node, export_pdf
```

## The crates

| Layer | Crate | Package | Responsibility |
|---|---|---|---|
| decode | `crates/decode/kiwi` | `grafta-kiwi` | Kiwi, Figma's binary schema format: schema and message decoding and encoding |
| | `crates/decode/schema` | `grafta-schema` | the format's field names, enum values and ids, each named once, and typed access to a decoded message |
| | `crates/decode/figfile` | `grafta-figfile` | the container: ZIP archive, `fig-kiwi` chunks (and `fig-deck`, `fig-jam.`, `fig-site`), blobs and images |
| | `crates/decode/model` | `grafta-model` | the typed scene graph: nodes, paints, variables, styles, components, vector paths; and authoring nodes back into it |
| core | `crates/core/domain` | `grafta-domain` | open files (`Session`, `Toolkit`), the ports, the product traits, value objects, domain events and errors. No I/O, JSON or transport crate |
| | `crates/core/app` | `grafta-app` | each tool as a typed `Command` or `Query`; the unit of work a command runs in; bounded results, views, auto-layout |
| products | `crates/products/design` | `grafta-design` | Figma Design: the base every other product extends |
| | `crates/products/slides` | `grafta-slides` | Figma Slides: slides in rows, transitions, animations, notes |
| | `crates/products/figjam` | `grafta-figjam` | FigJam: stickies, shapes, connectors, tables |
| | `crates/products/sites` | `grafta-sites` | Figma Sites: pages by route, each holding its breakpoints |
| adapters | `crates/adapters/disk` | `grafta-disk` | the ports on the machine: the file system, `figfile`, the journal on disk, `render` |
| | `crates/adapters/memory` | `grafta-memory` | the ports in memory, so tools, saves and replays are tested with no file on disk |
| | `crates/adapters/edge` | `grafta-edge` | where JSON meets grafta: each tool's name, description and schema, its arguments read into a command or query, results written as JSON, the journal |
| | `crates/toolkit` | `grafta-toolkit` | the assembly: the domain on the disk's ports, with the products and the edge |
| | `crates/render` | `grafta-render` | a node's subtree → SVG → PNG or PDF, offline, with notes on what is approximate |
| front ends | `crates/mcp` | `grafta-mcp` | the MCP server over the toolkit, on stdio |
| | `crates/cli` | `grafta` | the binary: `grafta mcp` and `grafta tool` |
| tests | `crates/testkit` | (not published) | scene graphs and fixture loading shared by the tests |

Each package publishes as `grafta-*` and keeps its short lib name, so
`use model::…` reads the same inside the workspace as outside it.

## The rules the layers keep

- **Layers point inward.** The domain and the application name no I/O, JSON
  or transport crate and no adapter; a product names no other product;
  `crates/toolkit/tests/layers.rs` reads every manifest to check it.
- **Ports, not calls.** The domain reaches files, the codec, the journal,
  images, the rasteriser and font measurement through traits
  (`DocumentStore`, `Codec`, `Journal`, `ImageSource`, `Rasterizer`,
  `JsonText`, `Measure` in `core/domain/src/port.rs`). `disk` implements them
  for real; `memory` for tests.
- **A product is a plugin.** `ProductProfile` (its container, name and
  summary) and `NodeBehavior` (what a node admits, how it is described, how its
  changes apply) are the domain's traits; `toolkit/src/product.rs` lists the
  products. Figma Sites was added as one crate and one line there.
- **The format is named once.** No literal from the file's format outside
  `schema`.
- **A command runs in a unit of work.** It commits whole, or rolls back whole;
  layout, instance caches and the journal are steps around it, and domain
  events (`NodeEvent`: a node moved, deleted or duplicated) let a product
  follow, inside the same unit.

## Adding a tool

Write its command or query in `core/app/src/tools/`, and its edge half in
`adapters/edge/src/` (implement `edge::Tool`: name, description, JSON Schema,
reading its arguments), then append it to `edge::ALL`. It then appears in
`grafta mcp`'s tool list and in `grafta tool` with no other wiring.
Regenerate the [tool reference](TOOLS.md) with
`python3 scripts/tool-reference.py target/release/grafta`; CI fails until you
do.

A tool that writes a kind of thing Figma has not yet opened from grafta is
proven with a **ladder**: a batch of rungs written to `ladder/`, opened by a
person in Figma. See
[CONTRIBUTING.md](CONTRIBUTING.md).

## Adding a product

A crate under `crates/products/` implementing `ProductProfile` and the
`NodeBehavior`s its node types need, and one line in
`crates/toolkit/src/product.rs`. No shared file of the domain or the
application changes.

## What grafta does not do

It does not interpret a design, and it does not invent Figma's format: the
schema always comes from a `.fig` (`new_file` takes it from one). It makes no
network call.
