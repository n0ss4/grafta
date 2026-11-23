// grafta's panel, its main thread: the file's side. A call from grafta is a
// tool's name and its arguments, already checked by grafta; it is done here
// through the plugin API, committed as one undo step, and answered in the
// tool's own result keys. What Figma refuses is answered in Figma's words. It
// decides nothing, and runs no code it is sent.
import { TOOLS } from "./contract";
import type { Call, Caller, FileInfo, ToMain, ToPanel } from "./protocol";
import { focus, planned, snapshot, target } from "./changes";
import * as protect from "./protect";
import { after, approveRest, approved, before as keepBefore, history, layersOf, picture, remember, requestFor, revert, show, type Request } from "./requests";
import { before, kindOf, said, tense } from "./said";
import { REGISTRY } from "./tools";

/** Set by the build: true in a development build, which has the self-test. */
declare const SELFTEST: boolean;

/** Where the secrets of the callers the person allowed are kept, on this computer. */
const ALLOWED = "allowed-callers";

/** Whether the person follows each change on the canvas, kept on this computer. */
const FOLLOW = "follow-changes";
let follow = true;
/** Whether each change waits for the person (Ask me first), kept on this computer. */
const ASK = "ask-first";
let ask = false;
/** How long a change waits for the person: grafta, told they are deciding, waits 120 s. */
const DECIDE_MS = 110_000;
const deciding = new Map<number, (decision: Decision) => void>();
let nextPending = 1;

interface Decision {
  apply: boolean;
  rest: boolean;
  late?: boolean;
}

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

/** Ask the person about a change, and wait: Apply, Skip, or no answer in time. */
async function decide(request: Request, call: Call, text: string): Promise<Decision> {
  const id = nextPending++;
  const base = await snapshot(call.args);
  const where = typeof call.args.id === "string" ? call.args.id : typeof call.args.parent === "string" ? call.args.parent : undefined;
  const until = Date.now() + DECIDE_MS;
  post({
    kind: "pending",
    pending: {
      id,
      connection: call.connection,
      call: call.id,
      caller: request.caller,
      text: tense(text, "asked"),
      planned: call.tool === "set_properties" && base ? planned(base, call.args.properties) : [],
      now: where ? await picture(where) : undefined,
      until,
    },
  });
  // Said on the canvas too, where the person is looking, with Apply at hand.
  const toast = figma.notify(`${request.caller} is waiting for you in grafta's panel`, {
    timeout: DECIDE_MS,
    button: { text: "Apply", action: () => deciding.get(id)?.({ apply: true, rest: false }) },
  });
  const decision = await new Promise<Decision>((resolve) => {
    deciding.set(id, resolve);
    setTimeout(() => resolve({ apply: false, rest: false, late: true }), DECIDE_MS);
  });
  deciding.delete(id);
  toast.cancel();
  post({ kind: "pending", pending: null });
  return decision;
}

async function answer(call: Call): Promise<void> {
  const started = Date.now();
  const reply: ToPanel = { kind: "answer", connection: call.connection, id: call.id, tool: call.tool, ms: 0 };
  const kind = kindOf(call.tool);
  const writes = kind !== "look";
  const layer = await before(call.args);
  const phrase = said(call.tool, call.args, undefined, undefined, layer).text;
  post({ kind: "now", text: tense(phrase, "going"), working: true });
  let request: Request | undefined;
  if (writes) {
    const why = await protect.refusal(call.args);
    if (why) reply.error = why;
    else {
      request = requestFor(call.caller ?? "An app");
      await keepBefore(request, call.args);
      if (ask && !approved(request)) {
        const decision = await decide(request, call, phrase);
        if (!decision.apply) {
          reply.error = decision.late
            ? "the person did not answer in Figma's panel in time (Ask me first): nothing was done"
            : "the person skipped this change in Figma's panel (Ask me first): nothing was done";
        } else if (decision.rest) approveRest(request);
      }
    }
  }
  if (reply.error === undefined) {
    try {
      reply.result = await run(call);
    } catch (error) {
      reply.error = words(error);
    }
    // One call, one step of Figma's undo.
    figma.commitUndo();
  }
  reply.ms = Date.now() - started;
  reply.said = said(call.tool, call.args, reply.result, reply.error, layer);
  if (reply.error === undefined && request) {
    await after(request, kind, call.args, reply.result);
    const shown = show(request);
    post({ kind: "request", request: shown });
    void remember(figma.root.name, shown);
    const id = target(call.args, reply.result);
    if (follow && id && kind !== "remove") await focus(id);
  }
  post({ kind: "now", text: reply.said.text, working: false });
  post(reply);
}

/** Select every layer a request made or changed, and bring them into view. */
async function showRequest(id: number): Promise<void> {
  const nodes = (await Promise.all(layersOf(id).map((key) => figma.getNodeByIdAsync(key).catch(() => null)))).filter(
    (n): n is SceneNode => Boolean(n) && !n!.removed && n!.type !== "PAGE" && n!.type !== "DOCUMENT",
  );
  if (!nodes.length) return;
  let page: BaseNode | null = nodes[0];
  while (page && page.type !== "PAGE") page = page.parent;
  if (page && page.type === "PAGE" && page !== figma.currentPage) await figma.setCurrentPageAsync(page);
  const here = nodes.filter((n) => {
    let at: BaseNode | null = n;
    while (at && at.type !== "PAGE") at = at.parent;
    return at === figma.currentPage;
  });
  figma.currentPage.selection = here;
  figma.viewport.scrollAndZoomIntoView(here);
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
    case "focus":
      void focus(message.id);
      break;
    case "follow":
      follow = message.on;
      void figma.clientStorage.setAsync(FOLLOW, follow);
      break;
    case "ask":
      ask = message.on;
      void figma.clientStorage.setAsync(ASK, ask);
      break;
    case "decide":
      deciding.get(message.id)?.({ apply: message.apply, rest: message.rest });
      break;
    case "revert":
      queue = queue.then(async () => {
        const shown = await revert(message.request);
        if (shown) {
          post({ kind: "request", request: shown });
          void remember(figma.root.name, shown);
        }
      });
      break;
    case "show":
      void showRequest(message.request);
      break;
    case "protect":
      post({ kind: "protected", list: protect.protectSelection() });
      break;
    case "unprotect":
      post({ kind: "protected", list: protect.unprotect(message.id) });
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
post({ kind: "protected", list: protect.list() });
void Promise.all([figma.clientStorage.getAsync(FOLLOW), figma.clientStorage.getAsync(ASK), history(figma.root.name)]).then(
  ([keptFollow, keptAsk, requests]) => {
    if (typeof keptFollow === "boolean") follow = keptFollow;
    if (typeof keptAsk === "boolean") ask = keptAsk;
    post({ kind: "follow", on: follow });
    post({ kind: "ask", on: ask });
    post({ kind: "history", requests });
  },
);
