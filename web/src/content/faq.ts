import type { FaqEntry } from "./types";

export const faq: FaqEntry[] = [
  {
    question: "Do I need a Figma account or an API key?",
    answer:
      "No. grafta reads and writes the file on your disk. You need Figma only to save a local copy (**File → Save local copy…**) and to open the result.",
  },
  {
    question: "Does my file leave my machine?",
    answer:
      "grafta makes no network call: it runs on your computer and works on the file in place. What your model reads through it is a tool result like any other, sent wherever your MCP client sends the conversation.",
  },
  {
    question: "Can it break my file?",
    answer:
      "It does not write over your files. `save_as` refuses a path that exists, with one exception: the empty file `new_file` just made, while it is still empty. It writes a new file and reads it back before it reports success, and refuses writes it knows Figma would have to repair. But grafta cannot run Figma: opening the result there is the final check, so keep your original.",
  },
  {
    question: "Which apps and models does it work with?",
    answer:
      "Any client that speaks MCP over stdio should work, with any model it runs. We have run it in Claude Code and Claude Desktop. Cursor, VS Code, Windsurf and Codex use the same setup, but we have not tried them yet. Clients that only connect to remote servers, such as ChatGPT, cannot use it yet.",
  },
  {
    question: "How accurate are the pictures?",
    answer:
      "Close, not Figma's own: `render_node` draws with the fonts on your machine, and with the design's font installed most lines lay out within 2% of Figma's width. Some things are not drawn exactly, such as two blend modes and noise effects, and an instance does not lay out again when an overridden text changes its width. The caption names most of what it approximated; see the [roadmap](https://github.com/n0ss4/grafta/blob/main/docs/ROADMAP.md).",
  },
  {
    question: "Which files can it open?",
    answer:
      "Figma Design `.fig`, Figma Slides `.deck`, FigJam `.jam` and Figma Sites `.site`, as Figma saves them. `new_file` starts an empty Design file in the format of one you have.",
  },
  {
    question: "Can I use it without a model?",
    answer:
      "Yes. `grafta tool` calls any tool from a terminal, and `grafta tool --batch` runs a sequence of calls on one file: this site's pictures, sessions and social card are made that way.",
  },
  {
    question: "What can't it do yet?",
    answer:
      "Auto-layout grids, component slots, boolean operations, prototyping, Dev Mode annotations and properties on variants. It edits only the variables and styles it made itself. It needs a `.fig` you have to start a new file, and an open file costs memory: 1.3 GB for a 57 MB file, 5.5 GB for an 86 MB one. See the [roadmap](https://github.com/n0ss4/grafta/blob/main/docs/ROADMAP.md).",
  },
  {
    question: "What if Figma changes its file format?",
    answer:
      "It may break. Figma does not document this format, and grafta was built by reading real files. A file carries its own schema, which helps, but a change could stop grafta reading a new file, or Figma accepting one it wrote. Report what you find in the issues.",
  },
  {
    question: "Which systems does it run on?",
    answer:
      "There are builds for macOS (Apple silicon and Intel), Linux (x86_64 and arm64) and Windows (x86_64). We have run it by hand on a Mac with Apple silicon; the tests also pass on Linux and Windows in CI. The binaries are not signed, so macOS or Windows may ask before the first run.",
  },
  {
    question: "What does it cost? Is it affiliated with Figma?",
    answer:
      "It is free and open source, under the MIT licence. It is not affiliated with, or endorsed by, Figma.",
  },
];
