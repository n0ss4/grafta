import type { Card } from "./types";

/** The split: what a person does. */
export const steps: Card[] = [
  {
    title: "Save a local copy",
    text: "In Figma, **File → Save local copy…** gives you a `.fig` (or a `.deck`, `.jam`, `.site`). That is all grafta needs.",
  },
  {
    title: "Ask in words",
    text: "“Open checkout.fig with grafta, give the payment screen a dark mode using colour variables, show me both, and save it as checkout-dark.fig.”",
  },
  {
    title: "Open the result",
    text: "Drag the new file into Figma. grafta saves beside yours and reads back what it wrote, but only Figma can say it opens.",
  },
];

export const promises: Card[] = [
  { title: "Exact", text: "What it reads comes from the file's bytes, with no network call of its own." },
  {
    title: "Says what it left out",
    text: "A cut result names what is missing; a picture, what it drew approximately.",
  },
  { title: "Does not write over your files", text: "Saves to a new path and re-reads what it wrote." },
  { title: "Survives restarts", text: "Unsaved edits are journaled as they are made, and replayed." },
  {
    title: "Refuses, not breaks",
    text: "It refuses writes it knows Figma would have to repair, and says why.",
  },
];

export const limits: Card[] = [
  {
    title: "It does not interpret designs",
    text: "Your model does. grafta answers what is in the file and writes what it is told.",
  },
  {
    title: "Files, not Figma's cloud",
    text: "It works on files you save locally. A new file takes Figma's format from a `.fig` you have.",
  },
  {
    title: "Pictures are close, not Figma's",
    text: "They use your machine's fonts. Two blend modes and noise effects are not drawn exactly, and in Preview some PDFs come out wrong.",
  },
  {
    title: "Open files cost memory",
    text: "A 57 MB file took 1.3 GB while open, and an 86 MB one took 5.5 GB. Close what you are done with.",
  },
  {
    title: "Not yet",
    text: "Auto-layout grids, component slots, boolean operations, prototyping, Dev Mode annotations, properties on variants.",
  },
  {
    title: "It cannot run Figma",
    text: "Opening the result in Figma is the final check, and yours to make. Keep your original.",
  },
  {
    title: "Its own tokens only",
    text: "The file's existing variables and styles are read and bound to; only grafta's own are edited.",
  },
];

/** How a model works with it: the loop. */
export const loop = [
  { verb: "Open", tool: "open_file", text: "a handle, kept open: later calls do not read the file again" },
  { verb: "Find", tool: "search_nodes", text: "pages, counts, nodes by name or type, with their path" },
  { verb: "Read", tool: "read_node", text: "a node exactly, in the plugin API's names" },
  { verb: "Write", tool: "set_properties", text: "frames, text, shapes, tokens, components" },
  { verb: "Look", tool: "render_node", text: "a picture, unsaved edits included" },
  { verb: "Save", tool: "save_as", text: "a new file, read back before it says done" },
] as const;
