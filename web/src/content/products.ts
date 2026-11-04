import type { Product } from "./types";

export const products: Product[] = [
  {
    name: "Figma Design",
    extension: ".fig",
    summary:
      "Screens, components and variants, variables with modes, styles, auto-layout, rich text, images and vectors.",
    roadmap: ["004", "005", "020", "026", "028"],
  },
  {
    name: "Figma Slides",
    extension: ".deck",
    summary:
      "Slides in their grid, speaker notes and templates; transitions and object animations, kept when slides move.",
    roadmap: ["049", "051", "052"],
  },
  {
    name: "FigJam",
    extension: ".jam",
    summary:
      "Stickies, shapes with text, connectors between nodes, sections, tables and stamps: a flow or a retro board.",
    roadmap: ["050", "053"],
  },
  {
    name: "Figma Sites",
    extension: ".site",
    summary:
      "A site's pages by route, each with its breakpoints; inside a breakpoint, every tool works as on a frame.",
    roadmap: ["062"],
  },
];
