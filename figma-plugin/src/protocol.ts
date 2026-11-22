// The messages between the panel (ui.ts), which holds the connections to
// grafta, and the plugin's main thread (main.ts), which holds the file.

/** Who a connection says it is, in its `hello`. */
export interface Caller {
  name: string;
  version: string;
  /** The shape of the messages it speaks (contract.json's `protocol`); none before there was one. */
  protocol?: number;
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
  /** The AI app's name, as its hello gave it. */
  caller?: string;
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

/** One property a call changed, as the person reads it: "Fill", "#FFFFFF", "#F59E0B". */
export interface Change {
  property: string;
  before: string;
  after: string;
}

/** A layer a request made, changed or removed, and what changed on it. */
export interface LayerView {
  id: string;
  name: string;
  made: boolean;
  removed: boolean;
  changes: Change[];
  more: number;
}

/** One request, as the panel shows it. */
export interface RequestView {
  id: number;
  started: number;
  ended: number;
  caller: string;
  made: number;
  changed: number;
  removed: number;
  layers: LayerView[];
  /** The frame it worked in, and its picture before and after, as PNG. */
  context?: string;
  before?: Uint8Array;
  after?: Uint8Array;
  reverted: boolean;
  revertable: boolean;
  notes: string[];
  /** From this session; a request from the file's history cannot be reverted. */
  live: boolean;
}

/** A change waiting for the person, in Ask me first. */
export interface PendingView {
  id: number;
  /** The connection and grafta's id for the call, so grafta is told the person is deciding. */
  connection: number;
  call: number;
  caller: string;
  /** "change the fill of “Card”". */
  text: string;
  planned: Change[];
  /** The frame now, as PNG. */
  now?: Uint8Array;
  /** When it is skipped on its own, as Date.now() gives it. */
  until: number;
}

export interface ProtectedView {
  id: string;
  name: string;
  type: string;
}

export type ToMain =
  | { kind: "known"; connection: number; caller: Caller }
  | { kind: "allow"; caller: Caller }
  | { kind: "call"; call: Call }
  | { kind: "selftest" }
  | { kind: "resize"; width: number; height: number; keep: boolean }
  | { kind: "focus"; id: string }
  | { kind: "follow"; on: boolean }
  | { kind: "ask"; on: boolean }
  | { kind: "decide"; id: number; apply: boolean; rest: boolean }
  | { kind: "revert"; request: number }
  | { kind: "show"; request: number }
  | { kind: "protect" }
  | { kind: "unprotect"; id: string };

export type ToPanel =
  | { kind: "file"; file: FileInfo; selftest?: boolean }
  | { kind: "selftest"; table: string }
  | { kind: "known"; connection: number; known: boolean; file: FileInfo }
  | { kind: "follow"; on: boolean }
  | { kind: "ask"; on: boolean }
  | { kind: "now"; text: string; working: boolean }
  | { kind: "request"; request: RequestView }
  | { kind: "history"; requests: RequestView[] }
  | { kind: "pending"; pending: PendingView | null }
  | { kind: "protected"; list: ProtectedView[] }
  | {
      kind: "answer";
      connection: number;
      id: number;
      tool: string;
      result?: unknown;
      error?: string;
      ms: number;
      said?: Said;
    };
