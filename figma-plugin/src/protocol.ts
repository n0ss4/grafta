// The messages between the panel (ui.ts), which holds the connections to
// grafta, and the plugin's main thread (main.ts), which holds the file.

/** Who a connection says it is, in its `hello`. */
export interface Caller {
  name: string;
  version: string;
  secret: string;
}

/** The file, as the panel names it to grafta. */
export interface FileInfo {
  name: string;
  editor: string;
}

/** A call from grafta, carried to the main thread. */
export interface Call {
  connection: number;
  id: number;
  tool: string;
  args: Record<string, unknown>;
}

/** What kind of thing a call did, for its icon. */
export type SaidKind = "look" | "make" | "change" | "move" | "remove";

/** A call, said in a designer's words (said.ts). */
export interface Said {
  kind: SaidKind;
  text: string;
  ok: boolean;
}

export type ToMain =
  | { kind: "known"; connection: number; caller: Caller }
  | { kind: "allow"; caller: Caller }
  | { kind: "call"; call: Call }
  | { kind: "selftest" }
  | { kind: "resize"; width: number; height: number; keep: boolean };

export type ToPanel =
  | { kind: "file"; file: FileInfo; selftest?: boolean }
  | { kind: "selftest"; table: string }
  | { kind: "known"; connection: number; known: boolean; file: FileInfo }
  | { kind: "answer"; connection: number; id: number; tool: string; result?: unknown; error?: string; ms: number; said?: Said };
