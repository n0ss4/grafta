// What the video shows inside the chat: the words, grafta's calls and the
// replies of a real session (web/public/sessions/talk.claude-code.json,
// Claude Code with claude-opus-5-5, 2026-11-07), shortened, never rewritten.
// The pictures are grafta's renders and the model's page, from web/public.

export const RECORDED = "Claude Code · claude-opus-5-5 · 7 Nov 2026";

/** The apps grafta documents, as web/src/content/clients.ts names them. */
export const APPS = [
  { name: "Claude Desktop", short: "Cl", tint: "#D97757" },
  { name: "Claude Code", short: ">_", tint: "#1F1E1D" },
  { name: "ChatGPT", short: "GP", tint: "#0F0F0F" },
  { name: "Codex", short: "Cx", tint: "#3B3B3F" },
  { name: "Cursor", short: "Cu", tint: "#141414" },
  { name: "VS Code", short: "VS", tint: "#1F6FB5" },
  { name: "Windsurf", short: "Ws", tint: "#0B6E63" },
] as const;

export interface Exchange {
  words: string;
  steps: { label: string; tools: string[] }[];
  lead: string;
}

export const EXCHANGES: Exchange[] = [
  {
    words: "What's in order.fig?",
    steps: [
      { label: "Opened the file", tools: ["open_file"] },
      { label: "Read its layers, variables and a picture", tools: ["read_node", "list_variables", "render_node"] },
    ],
    lead: "One small mobile screen: an order-status card in a dark theme. A frame “Order”, 360 wide, auto-layout with 24px padding, set in Inter. Every fill is bound to one of 9 colours in a “Theme” collection, with Light and Dark modes.",
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
    lead: "Nothing points to a framework, so it's plain HTML and CSS, with both themes as CSS variables: order.html.",
  },
];

/** The contrast reply's table, as written (4 of its 6 rows). */
export const CONTRAST = [
  { text: "Order #4821", pair: ["#15171C", "#FFFFFF"], ratio: "17.9", needs: "4.5" },
  { text: "Out for delivery", pair: ["#13704B", "#FFFFFF"], ratio: "6.09", needs: "4.5" },
  { text: "Track order", pair: ["#FFFFFF", "#13704B"], ratio: "6.09", needs: "4.5" },
  { text: "Placed 2 Oct · 3 items", pair: ["#6B6F76", "#F6F4EE"], ratio: "4.58", needs: "4.5" },
];

/** Lines of the model's order.html, picked, in its order. */
export const CODE = `:root {
  --background: #F6F4EE;
  --ink: #15171C;
  --accent: #13704B;
}
[data-theme="dark"] {
  --background: #0D1117;
  --accent: #C6F26B;
}
.order {
  width: 360px;
  display: flex;
  gap: 20px;
  padding: 24px;
}`;

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
