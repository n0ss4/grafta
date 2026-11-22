// grafta's panel, its main thread: the file's side. A call from grafta is a
// tool's name and its arguments, already checked by grafta; it is done here
// through the plugin API, committed as one undo step, and answered in the
// tool's own result keys. What Figma refuses is answered in Figma's words. It
// decides nothing, and runs no code it is sent.
import { TOOLS } from "./contract";
import type { Call, Caller, FileInfo, ToMain, ToPanel } from "./protocol";
import { before, said } from "./said";
import { REGISTRY } from "./tools";

/** Set by the build: true in a development build, which has the self-test. */
declare const SELFTEST: boolean;

/** Where the secrets of the callers the person allowed are kept, on this computer. */
const ALLOWED = "allowed-callers";

/** Where the size the person last gave the panel is kept, on this computer. */
const SIZE = "panel-size";
/** The panel's size, and the smallest and largest it is let be. */
const WIDTH = 320;
const HEIGHT = 440;
const MIN = { width: 280, height: 320 };
const MAX = { width: 800, height: 1200 };

figma.showUI(__html__, { width: WIDTH, height: HEIGHT, themeColors: true, title: "grafta" });

function fit(width: number, height: number): { width: number; height: number } {
  const clamp = (value: number, low: number, high: number) => Math.round(Math.min(high, Math.max(low, value)));
  return { width: clamp(width, MIN.width, MAX.width), height: clamp(height, MIN.height, MAX.height) };
}

void figma.clientStorage.getAsync(SIZE).then((kept: unknown) => {
  const size = kept as { width?: unknown; height?: unknown } | undefined;
  if (typeof size?.width === "number" && typeof size.height === "number") {
    const { width, height } = fit(size.width, size.height);
    figma.ui.resize(width, height);
  }
});

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
  const layer = await before(call.args);
  try {
    reply.result = await run(call);
  } catch (error) {
    reply.error = words(error);
  }
  // One call, one step of Figma's undo.
  figma.commitUndo();
  reply.ms = Date.now() - started;
  reply.said = said(call.tool, call.args, reply.result, reply.error, layer);
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
    case "resize": {
      const { width, height } = fit(message.width, message.height);
      figma.ui.resize(width, height);
      if (message.keep) void figma.clientStorage.setAsync(SIZE, { width, height });
      break;
    }
    case "selftest":
      if (SELFTEST) queue = queue.then(runSelftest);
      break;
  }
};

/** The self-test, in the queue like a call, so no call interleaves with it. */
async function runSelftest(): Promise<void> {
  const { selftest, table } = await import("./selftest");
  try {
    const { rows, left } = await selftest();
    post({ kind: "selftest", table: table(rows, left) });
  } catch (error) {
    post({ kind: "selftest", table: `The self-test stopped: ${words(error)}` });
  }
  figma.commitUndo();
}

post({ kind: "file", file: file(), selftest: SELFTEST });
