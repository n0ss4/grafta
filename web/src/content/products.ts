import type { Product } from "./types";

export const products: Product[] = [
  {
    name: "Figma Design",
    extension: ".fig",
    summary:
      "Screens, components and variants, variables with modes, styles, auto-layout, rich text, images and vectors.",
  },
  {
    name: "Figma Slides",
    extension: ".deck",
    summary:
      "Slides in their grid, speaker notes and templates; transitions and object animations, kept when slides move.",
  },
  {
    name: "FigJam",
    extension: ".jam",
    summary:
      "Stickies, shapes with text, connectors between nodes, sections, tables and stamps: a flow or a retro board.",
  },
  {
    name: "Figma Sites",
    extension: ".site",
    summary:
      "A site's pages by route, each with its breakpoints; inside a breakpoint, every tool works as on a frame.",
  },
];
