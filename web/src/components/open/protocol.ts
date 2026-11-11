// The messages between the page and the worker that runs grafta's reader
// (internal: specs/109-open-in-browser/contracts/browser-reader.md).

export type ToWorker =
  | { type: "open"; id: number; name: string; bytes: ArrayBuffer }
  | { type: "call"; id: number; tool: string; args: object }
  | { type: "picture"; id: number; args: object }
  | { type: "font"; id: number; bytes: ArrayBuffer }
  | { type: "take"; id: number; path: string };

export type FromWorker =
  | { type: "ready" }
  | { type: "result"; id: number; result: unknown }
  | { type: "picture"; id: number; png: ArrayBuffer; caption: Caption }
  | { type: "file"; id: number; bytes: ArrayBuffer }
  | { type: "error"; id?: number; message: string; fatal: boolean };

/** What render_node says beside a picture. */
export interface Caption {
  id: string;
  name: string;
  width: number;
  height: number;
  scale: number;
  notes: string[];
}

/** summarize's answer, as far as the page reads it. */
export interface Summary {
  file: string;
  handle: string;
  product: "design" | "slides" | "whiteboard" | "site";
  nodes: number;
  pages: { id: string; name: string; nodes: number; internalOnly?: boolean }[];
  variables: number;
  variable_collections: number;
}

export interface Screen {
  id: string;
  name: string;
  type: string;
  width: number;
  height: number;
  visible: boolean;
}

export interface Screens {
  page: { id: string; name: string };
  results: Screen[];
  total: number;
  next_cursor?: string;
}

export type VariableValue =
  string | number | boolean | { alias: string } | { color: string; opacity: number };

export interface Variables {
  collections: { id: string; name: string; modes: string[]; variables: number }[];
  results: {
    id: string;
    name: string;
    collection: string;
    type: string;
    values: Record<string, VariableValue>;
  }[];
  next_cursor?: string;
}

export interface Tokens {
  files: { path: string; collection: string; mode: string; tokens: number }[];
  unresolved: { variable: string; mode: string; alias: string }[];
  renamed: { from: string; to: string }[];
  notes: string[];
}
