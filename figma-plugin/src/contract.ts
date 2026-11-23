// What this plugin agrees on with grafta, from contract.json, which grafta
// writes (`GRAFTA_RECORD=1 cargo test -p grafta-mcp`); nothing here is
// written by hand.
import { protocol, tools } from "./contract.json";

/** The shape of the messages this panel speaks with grafta. */
export const PROTOCOL: number = protocol;

/** What a tool does on the open file. */
export type Live = "same" | "differs" | "cannot";

export interface ToolContract {
  name: string;
  live: Live;
  why?: string;
}

/** Every tool grafta has, with what it does here. */
export const TOOLS: readonly ToolContract[] = tools.map((tool) => ({
  name: tool.name,
  live: tool.live as Live,
  why: "why" in tool ? String(tool.why) : undefined,
}));
