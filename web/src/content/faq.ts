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
      "It never overwrites one. `save_as` refuses a path that exists, writes a new file, and reads it back before it reports success. It refuses what Figma would have to repair, and unsaved edits survive a restart.",
  },
  {
    question: "Which apps and models does it work with?",
    answer:
      "Any client that speaks MCP over stdio: Claude Desktop (a one-click bundle), Claude Code (a plugin that also teaches the model the workflow), Cursor, VS Code, Windsurf and others, with any model they run.",
  },
  {
    question: "How accurate are the pictures?",
    answer:
      "Close, not Figma's own: `render_node` draws with the fonts on your machine. With the design's font installed, most lines lay out within 2% of Figma's width; without it, the caption says a stand-in was used. Install the font and restart the server.",
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
    question: "What does it cost? Is it affiliated with Figma?",
    answer:
      "It is free and open source, under the MIT licence. It is not affiliated with, or endorsed by, Figma.",
  },
];
