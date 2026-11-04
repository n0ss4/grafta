import type { Prompt } from "./types";

export const prompts: Prompt[] = [
  {
    job: "Add a dark mode",
    text: "Open app.fig with grafta. Turn the colours on the Settings screen into Light and Dark variables, bind every fill to them, render both modes, and save as app-dark.fig.",
    tools: ["create_variable", "set_properties", "render_node"],
  },
  {
    job: "Audit a design system",
    text: "Open library.fig with grafta. List its components, variables and styles, and tell me which colours are used on screens without being a variable or a style.",
    tools: ["list_components", "list_variables", "search_nodes"],
  },
  {
    job: "Check accessibility",
    text: "Open checkout.fig with grafta and check every screen's text contrast against WCAG AA. Fix what fails with the file's own colours, and show me before and after.",
    tools: ["check_contrast", "set_properties", "render_node"],
  },
  {
    job: "Fill screens from data",
    text: "Open shop.fig with grafta. Place the Product card component once per row of products.csv in a wrapping grid, with each row's name, price and photo.",
    tools: ["fill_template", "create_frame"],
  },
  {
    job: "Turn a web page into layers",
    text: "Import page.json, a snapshot of our landing page, into a new page of site.fig with grafta, so the design team can edit it with auto-layout.",
    tools: ["import_layout", "save_as"],
  },
  {
    job: "Build a deck or a board",
    text: "Open pitch.deck with grafta. Add three slides after the title with our Q3 numbers, a fade between them, and speaker notes; then export every slide as one PDF.",
    tools: ["duplicate_node", "set_properties", "export_pdf"],
  },
];
