// grafta's panel, its main thread: the file's side. A call from grafta is a
// tool's name and its arguments, already checked by grafta; it is done here
// through the plugin API, committed as one undo step, and answered in the
// tool's own result keys. What Figma refuses is answered in Figma's words. It
// decides nothing, and runs no code it is sent.
import { TOOLS } from "./contract";
import type { Call, Caller, FileInfo, ToMain, ToPanel } from "./protocol";
import { REGISTRY } from "./tools";

/** Where the secrets of the callers the person allowed are kept, on this computer. */
const ALLOWED = "allowed-callers";

figma.showUI(__html__, { width: 320, height: 280, themeColors: true, title: "grafta" });

function post(message: ToPanel): void {
  figma.ui.postMessage(message);
}

function file(): FileInfo {
  return { name: figma.root.name, editor: figma.editorType };
}

async function allowed(): Promise<string[]> {
  const kept: unknown = await figma.clientStorage.getAsync(ALLOWED);
  return Array.isArray(kept) ? kept.filter((secret): secret is string => typeof secret === "string") : [];
}

async function allow(caller: Caller): Promise<void> {
  const kept = await allowed();
  if (!kept.includes(caller.secret)) await figma.clientStorage.setAsync(ALLOWED, [...kept, caller.secret]);
}

/** The words of whatever was thrown: Figma's own, unchanged. */
function words(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function run(call: Call): Promise<unknown> {
  const tool = TOOLS.find((t) => t.name === call.tool);
  if (!tool) throw new Error(`this panel knows no tool ${call.tool}: grafta and the panel are of different versions`);
  if (tool.live === "cannot") throw new Error(`${call.tool} does not act on a file open in Figma: ${tool.why ?? ""}`);
  const done = REGISTRY[call.tool];
  if (!done) throw new Error(`this panel has no ${call.tool}: update grafta's panel to the version of grafta you run`);
  return done(call.args);
}

async function answer(call: Call): Promise<void> {
  const started = Date.now();
  const reply: ToPanel = { kind: "answer", connection: call.connection, id: call.id, tool: call.tool, ms: 0 };
  try {
    reply.result = await run(call);
  } catch (error) {
    reply.error = words(error);
  }
  // One call, one step of Figma's undo.
  figma.commitUndo();
  reply.ms = Date.now() - started;
  post(reply);
}

// One call at a time, in the order they came: two calls never interleave
// their steps in the file.
let queue: Promise<void> = Promise.resolve();

figma.ui.onmessage = (message: ToMain) => {
  switch (message.kind) {
    case "known":
      void allowed().then((kept) =>
        post({ kind: "known", connection: message.connection, known: kept.includes(message.caller.secret), file: file() }),
      );
      break;
    case "allow":
      void allow(message.caller);
      break;
    case "call":
      queue = queue.then(() => answer(message.call));
      break;
  }
};

post({ kind: "file", file: file() });
