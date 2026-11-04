// A session as one client recorded it.
import type { Step } from "../live/types";

export type { Step };

export type ClientId = "claude-desktop" | "claude-code";

export const clientName: Record<ClientId, string> = {
  "claude-desktop": "Claude Desktop",
  "claude-code": "Claude Code",
};

export type Turn =
  | { kind: "user"; text: string }
  | { kind: "reply"; text: string }
  | {
      kind: "tools";
      /** The row's words: "Added a High contrast mode". */
      label: string;
      /** The beat of the session these calls reach (content/sessions.json). */
      beat: string;
      /** The client's own tools, not grafta's: shown collapsed, never dropped. */
      other?: boolean | undefined;
      calls: Step[];
    };

export interface Recording {
  id: string;
  client: ClientId;
  clientVersion: string;
  model: string;
  recordedOn: string;
  /** The file the session wrote, from its last save_as; null when it wrote none. */
  file: string | null;
  edits: { paths: boolean; cut: number; otherTools: number; pictures: number };
  turns: Turn[];
}
