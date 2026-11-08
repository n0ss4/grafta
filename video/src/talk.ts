// What the video shows inside the chat: the words, grafta's calls and the
// replies of a real session (web/public/sessions/talk.claude-code.json,
// Claude Code with claude-opus-5-5, 2026-11-07), shortened, never rewritten.
// The code is the file the model wrote, whole, read from the recording.
// The pictures are grafta's renders and the model's page, from web/public.

import recording from "../../web/public/sessions/talk.claude-code.json";

export const RECORDED = "Claude Code · claude-opus-5-5 · 7 Nov 2026";
/** The same requests in Claude Desktop, on 8 Nov 2026, as the maintainer ran them:
 * where it differs from the recording, the scene follows the app (the upload
 * copied to the Mac, the code as a card). */
export const DESKTOP = "Claude Desktop · 8 Nov 2026";

/** The apps grafta documents, as web/src/content/clients.ts names them, with their own marks (assets/logos). */
export const APPS: { name: string; logo: string; tile?: string; pad?: number; invert?: boolean }[] = [
  { name: "Claude Desktop", logo: "logos/claude.png" },
  { name: "Claude Code", logo: "logos/claudecode.svg", tile: "#D97757", pad: 30, invert: true },
  { name: "ChatGPT", logo: "logos/chatgpt.png" },
  { name: "Codex", logo: "logos/chatgpt.png", tile: "#0D0D0D", pad: 0, invert: true },
  { name: "Cursor", logo: "logos/cursor.svg", tile: "#141414", pad: 30, invert: true },
  { name: "VS Code", logo: "logos/visual-studio-code.png" },
  { name: "Windsurf", logo: "logos/windsurf.svg", tile: "#0B100F", pad: 30, invert: true },
];

export interface Exchange {
  words: string;
  steps: { label: string; tools: string[] }[];
  lead: string;
  /** A second paragraph, from the Claude Desktop session. */
  note?: string;
}

export const EXCHANGES: Exchange[] = [
  {
    words: "What's in order.fig?",
    steps: [
      { label: "Opened the file", tools: ["open_file"] },
      { label: "Read its layers, variables and a picture", tools: ["read_node", "list_variables", "render_node"] },
    ],
    lead: "One small mobile screen: an order-status card in a dark theme. A frame “Order”, 360 wide, auto-layout with 24px padding, set in Inter. Every fill is bound to one of 9 colours in a “Theme” collection, with Light and Dark modes.",
    note: "Grafta runs on your Mac, so to open the file I saved a copy to your Downloads folder as order.fig.",
  },
  {
    words: "Make it light, and check the contrast.",
    steps: [
      { label: "Set the Light mode and checked every text", tools: ["set_properties", "check_contrast"] },
      { label: "Drew it and saved a new file", tools: ["render_node", "save_as"] },
    ],
    lead: "I switched the Order frame to the Light mode and saved it as a new file, order-light.fig. The original hasn't changed.",
  },
  {
    words: "Now turn it into code.",
    steps: [{ label: "Read the exact values of its icons", tools: ["read_node", "read_node", "read_node"] }],
    lead: "I turned the screen into a single HTML file, order.html. Nothing pointed to a framework, so it's plain HTML and CSS, with both themes as CSS variables.",
  },
];

/** The contrast reply's table, as written (4 of its 6 rows). */
export const CONTRAST = [
  { text: "Order #4821", pair: ["#15171C", "#FFFFFF"], ratio: "17.9", needs: "4.5" },
  { text: "Out for delivery", pair: ["#13704B", "#FFFFFF"], ratio: "6.09", needs: "4.5" },
  { text: "Track order", pair: ["#FFFFFF", "#13704B"], ratio: "6.09", needs: "4.5" },
  { text: "Placed 2 Oct · 3 items", pair: ["#6B6F76", "#F6F4EE"], ratio: "4.58", needs: "4.5" },
];

/** The page the model wrote, order.html, as its Write and Edit calls left it. */
export const ORDER_HTML: string = (() => {
  let content = "";
  for (const turn of recording.turns) {
    if (turn.kind !== "tools" || !turn.other) continue;
    for (const call of turn.calls) {
      const args = call.args as Record<string, unknown>;
      if (call.tool === "Write" && typeof args.content === "string") content = args.content;
      if (call.tool === "Edit" && typeof args.old_string === "string" && typeof args.new_string === "string") {
        content = content.replace(args.old_string, args.new_string);
      }
    }
  }
  if (!content) throw new Error("talk: the recording has no Write of order.html");
  return content;
})();

/** order-light.fig's layers, from web/public/showcase/talk/layers.json. */
export const LAYERS: { name: string; type: "FRAME" | "TEXT" | "VECTOR"; depth: number }[] = [
  { name: "Order", type: "FRAME", depth: 0 },
  { name: "My order", type: "TEXT", depth: 1 },
  { name: "Placed 2 Oct · 3 items", type: "TEXT", depth: 1 },
  { name: "Card", type: "FRAME", depth: 1 },
  { name: "Header", type: "FRAME", depth: 2 },
  { name: "Icon", type: "FRAME", depth: 3 },
  { name: "Labels", type: "FRAME", depth: 3 },
  { name: "Progress", type: "FRAME", depth: 2 },
  { name: "Steps", type: "FRAME", depth: 2 },
  { name: "Button", type: "FRAME", depth: 1 },
  { name: "Track order", type: "TEXT", depth: 2 },
  { name: "Arrow (Lucide)", type: "VECTOR", depth: 2 },
  { name: "Delivered to the door · no signature", type: "TEXT", depth: 1 },
];
