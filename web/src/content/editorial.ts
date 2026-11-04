// The home page's words in the editorial direction. Every figure says
// where it was measured.

export const principles = [
  { title: "Exact", text: "Every figure from the file's bytes. No heuristics, no model, no estimate." },
  { title: "Offline", text: "No account, no API key, no network call. Your files stay yours." },
  { title: "Whole", text: "Design, Slides, FigJam and Sites files: one toolkit, the same calls." },
  { title: "Honest", text: "A cut result names what is missing; a picture, what it drew approximately." },
] as const;

/** Figma's MCP server as its documentation describes it on 30 October 2026 (help.figma.com, developers.figma.com). */
export const comparison = [
  { aspect: "Account", grafta: "None", figma: "A Figma account; a Full seat to write outside drafts" },
  { aspect: "Where", grafta: "Files on your disk", figma: "Files in Figma, live in the app" },
  { aspect: "Network", grafta: "None: it runs offline", figma: "Through Figma's servers" },
  {
    aspect: "Cost",
    grafta: "None: no plan, no quota",
    figma: "Tool calls per plan; writing free in beta, then usage-based",
  },
  { aspect: "Source", grafta: "Open source, MIT", figma: "Figma's own" },
  {
    aspect: "Best for",
    grafta: "Batch work, CI, big files, any agent",
    figma: "Working live with a file open in Figma",
  },
] as const;

export const jobs = [
  {
    title: "Audit a design system",
    session: "audit",
    prompt: "Audit checkout.fig: its tokens, components and styles, and the text not bound to them.",
    tools: ["summarize", "list_variables", "list_styles"],
  },
  {
    title: "Theme it",
    session: "theme",
    prompt: "Add a High contrast mode to the Theme and show me every screen in it.",
    tools: ["set_variable_collection", "set_variable", "render_node"],
  },
  {
    title: "Fix contrast",
    session: "contrast",
    prompt: "Check every text against WCAG AA and fix what fails with the file's own colours.",
    tools: ["check_contrast", "set_properties"],
  },
  {
    title: "Fill from data",
    session: "data",
    prompt: "Make the product card a component and place it once per row of products.csv.",
    tools: ["create_component", "fill_template"],
  },
  {
    title: "Bring in a web page",
    session: "html",
    prompt: "Import this page's snapshot as layers the design team can edit.",
    tools: ["import_layout"],
  },
  {
    title: "Build the deck",
    session: null,
    prompt: "Add three slides with our Q3 numbers, a fade between them, and speaker notes.",
    tools: ["duplicate_node", "set_properties"],
  },
] as const;

/** Measured designing this page with grafta, on the approved sixth draft. */
export const proof = [
  { value: 64579, shown: "64,579", label: "nodes of a UI kit, opened in 1.3 s" },
  { value: 167, shown: "167", label: "texts checked against WCAG AA; one fails, the decorative wordmark" },
  { value: 110, shown: "110", label: "kit pages removed, so the file holds this design alone" },
  { value: 10.9, shown: "10.9 MB", label: "from 132 MB, once the kit's unused images were dropped" },
] as const;
