# Examples

## Four products

The same tools open, read, draw and save every file Figma saves, and each
product's files were opened in that product to prove it.

| Product | File | What grafta speaks |
|---|---|---|
| **Figma Design** | `.fig` | screens, components and variants, variables with modes, styles, auto-layout, rich text, images, vectors |
| **Figma Slides** | `.deck` | slides in their grid, speaker notes, templates; transitions and object animations, kept when slides move |
| **FigJam** | `.jam` | stickies, shapes with text, connectors between nodes, sections, tables, stamps |
| **Figma Sites** | `.site` | a site's pages by route, each holding its breakpoints; inside a breakpoint, every tool works as on a frame |

## See it

The same frame, three times. Each picture is grafta's own `render_node`; every
change between them is a tool call a model would make
([58 of them](../showcase/showcase.py)).

| A draft | Designed | Dark mode |
|:---:|:---:|:---:|
| <img src="docs/assets/showcase-1-draft.png" width="260" alt="A grey wireframe of an order card"> | <img src="docs/assets/showcase-2-designed.png" width="260" alt="The same card with colour tokens, an icon, a progress bar and a green button"> | <img src="docs/assets/showcase-3-dark.png" width="260" alt="The same card in its Dark mode"> |
| frames and text | 9 colour variables, Lucide icons, auto-layout | one property on the frame: `explicitVariableModes` |

## From Figma to json-render

A screen in a Figma file, written as a [json-render](https://github.com/vercel-labs/json-render)
spec, the JSON an AI writes and json-render draws with your components. Left, the
*Edit profile* dialog in the shadcn/ui Figma kit; right, the spec your AI wrote from
it with grafta, drawn by json-render's shadcn components.

| In Figma | Drawn by json-render |
|:---:|:---:|
| <img src="docs/assets/json-render-figma.png" width="320" alt="The Edit profile dialog in the shadcn/ui Figma kit"> | <img src="docs/assets/json-render-drawn.png" width="320" alt="The same dialog drawn by json-render with its shadcn components"> |

How, step by step, and what does not match: [From Figma to json-render](json-render.md).

