// grafta's panel, its window: the connections to grafta on this computer, one
// to each grafta that answers on its ports, and what the person sees and
// decides. It sends nothing anywhere but to those connections.
import { PROTOCOL } from "./contract";
import { PORTS } from "./ports";
import type { Caller, Call, FileInfo, Said, SaidKind, ToMain, ToPanel } from "./protocol";

/** How often a port with nothing on it is tried again. */
const RETRY_MS = 1000;
/** How many calls done are kept, the list scrolling past what fits. */
const LAST = 50;

interface Connection {
  number: number;
  port: number;
  socket: WebSocket;
  caller?: Caller;
  allowed: boolean;
  /** Why this grafta and this panel cannot work together, when they speak another protocol. */
  mismatch?: string;
}

const connections = new Map<number, Connection>();
const trying = new Set<number>();
/** Callers refused in this session, by secret: not asked about again. */
const refused = new Set<string>();
/** Connections waiting for the person to allow or refuse them. */
const asking: Connection[] = [];
interface Done {
  said: Said;
  /** Figma's words, when it refused. */
  error?: string;
  /** The tools behind the row, newest first: several when looks are merged. */
  tools: string[];
  ms: number;
}

/** A development build: rows also give the tool and its time. */
let dev = false;
/** Bumped when the calls done change; the list is drawn again only then, so it does not flicker. */
let edits = 0;
let drawn = -1;

const last: Done[] = [];
let file: FileInfo = { name: "", editor: "figma" };
let stopped = false;
let next = 1;

const element = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

function toMain(message: ToMain): void {
  parent.postMessage({ pluginMessage: message }, "*");
}

function send(connection: Connection, value: unknown): void {
  if (connection.socket.readyState === WebSocket.OPEN) connection.socket.send(JSON.stringify(value));
}

type State = "searching" | "found" | "mismatch" | "connected" | "stopped";

const SAID: Record<State, { pill: string; sub: string }> = {
  searching: { pill: "Searching", sub: "Start grafta in your AI app: this panel finds it on its own." },
  found: { pill: "Found", sub: "grafta is running here. Waiting for the app to say who it is." },
  mismatch: { pill: "Update needed", sub: "" },
  connected: { pill: "Connected", sub: "Ask your AI app to read or change this file. Each call is one step of undo." },
  stopped: { pill: "Stopped", sub: "Every call is refused until you resume." },
};

function render(): void {
  const open = [...connections.values()];
  const allowed = open.filter((c) => c.allowed);
  const mismatched = open.find((c) => c.mismatch);
  const usable = open.filter((c) => !c.mismatch);
  const state: State = stopped
    ? "stopped"
    : allowed.length
      ? "connected"
      : usable.length
        ? "found"
        : mismatched
          ? "mismatch"
          : "searching";
  document.body.dataset.state = state;
  element("pill").textContent = SAID[state].pill;
  element("state-sub").textContent =
    state === "found" && asking.length
      ? "An app is asking to use this file: allow it below, or refuse it."
      : state === "mismatch"
        ? (mismatched?.mismatch ?? "")
        : SAID[state].sub;
  element("state").textContent = stopped
    ? "Stopped: grafta can do nothing here"
    : allowed.length
      ? `Connected to ${allowed.length === 1 ? (allowed[0].caller?.name ?? "an app") : `${allowed.length} apps`}`
      : usable.length
        ? "grafta found on this computer"
        : mismatched
          ? `grafta ${mismatched.caller?.version ?? ""} and this panel do not match`
          : "Looking for grafta on this computer…";
  const list = element<HTMLUListElement>("callers");
  list.replaceChildren(...allowed.map((c) => chip(c.caller?.name ?? "an app", c.caller?.version)));
  // One app is named in the state above; the list is for two or more.
  element("callers-section").hidden = allowed.length < 2;
  const waiting = asking[0];
  element("ask").hidden = !waiting;
  if (waiting) element("ask-name").textContent = waiting.caller?.name ?? "An app";
  if (drawn !== edits) {
    element<HTMLUListElement>("last").replaceChildren(...last.map(row));
    drawn = edits;
  }
  element("last").hidden = last.length === 0;
  element("last-empty").hidden = last.length > 0;
  element("clear").hidden = last.length === 0;
  element("stop-text").textContent = stopped ? "Resume" : "Stop";
}

function doing(tool: string | undefined): void {
  element("doing-wrap").hidden = tool === undefined;
  element("doing").textContent = tool === undefined ? "" : dev ? tool : "Working…";
}

function chip(name: string, version: string | undefined): HTMLLIElement {
  const li = document.createElement("li");
  li.className = "chip";
  li.title = `${name} · grafta ${version ?? ""}`;
  const dot = document.createElement("i");
  dot.className = "dot";
  const label = document.createElement("span");
  label.textContent = name;
  li.append(dot, label);
  if (version) {
    const v = document.createElement("span");
    v.className = "v";
    v.textContent = version;
    li.append(v);
  }
  return li;
}

const SVG = "http://www.w3.org/2000/svg";
/** One icon per kind of call, as SVG paths on a 24 grid. */
const ICONS: Record<SaidKind | "failed", string> = {
  look: "M2.5 12s3.5-6.5 9.5-6.5S21.5 12 21.5 12s-3.5 6.5-9.5 6.5S2.5 12 2.5 12zM12 14.75a2.75 2.75 0 1 0 0-5.5 2.75 2.75 0 0 0 0 5.5z",
  make: "M12 5v14M5 12h14",
  change: "M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4",
  move: "M12 3v18M3 12h18M12 3l-3 3M12 3l3 3M12 21l-3-3M12 21l3-3M3 12l3-3M3 12l3 3M21 12l-3-3M21 12l-3 3",
  remove: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
  failed: "M12 8v5M12 16.5v.01M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z",
};

function icon(kind: SaidKind | "failed"): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", ICONS[kind]);
  for (const [k, v] of Object.entries({ fill: "none", stroke: "currentColor", "stroke-width": "2", "stroke-linecap": "round", "stroke-linejoin": "round" }))
    path.setAttribute(k, v);
  svg.append(path);
  return svg;
}

function span(className: string, text: string): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = className;
  el.textContent = text;
  return el;
}

function row(done: Done): HTMLLIElement {
  const li = document.createElement("li");
  li.className = done.said.ok ? done.said.kind : "failed";
  li.title = done.tools.join(", ");
  const badge = document.createElement("span");
  badge.className = "kind";
  badge.append(icon(done.said.ok ? done.said.kind : "failed"));
  const body = document.createElement("span");
  body.className = "body";
  body.append(span("text", done.said.text));
  if (done.error) body.append(span("why", done.error));
  if (dev) body.append(span("tech", `${done.tools[0]} · ${done.ms} ms`));
  li.append(badge, body);
  if (done.tools.length > 1) li.append(span("count", `×${done.tools.length}`));
  return li;
}

/** A call done goes on top; looks one after another share a row, so changes stay in sight. */
function record(done: Done): void {
  edits++;
  const top = last[0];
  if (top && done.said.kind === "look" && done.said.ok && top.said.kind === "look" && top.said.ok) {
    top.tools.unshift(...done.tools);
    top.said = { ...top.said, text: "Looked through the file" };
    top.ms = done.ms;
    return;
  }
  last.unshift(done);
  last.length = Math.min(last.length, LAST);
}

function grant(connection: Connection): void {
  connection.allowed = true;
  send(connection, { allowed: true, file });
  if (stopped) send(connection, { stopped: true });
}

function receive(connection: Connection, text: string): void {
  let message: Record<string, unknown>;
  try {
    message = JSON.parse(text) as Record<string, unknown>;
  } catch {
    return;
  }
  const hello = message.hello as Caller | undefined;
  if (hello) {
    connection.caller = hello;
    connection.mismatch = mismatch(hello);
    // Nothing is sent: the connection stays open, so the panel does not
    // reconnect each second, and grafta lists no file until one is updated.
    if (connection.mismatch) {
      render();
      return;
    }
    if (refused.has(hello.secret)) {
      send(connection, { allowed: false, why: "the person did not allow it" });
      connection.socket.close();
    } else if (connection.allowed) {
      render();
    } else {
      toMain({ kind: "known", connection: connection.number, caller: hello });
    }
    return;
  }
  if (typeof message.id === "number" && typeof message.tool === "string") {
    if (!connection.allowed) return;
    if (stopped) {
      send(connection, { id: message.id, error: "the person stopped grafta in Figma's panel: nothing was done", ms: 0 });
      return;
    }
    doing(message.tool);
    const call: Call = { connection: connection.number, id: message.id, tool: message.tool, args: (message.args ?? {}) as Record<string, unknown> };
    toMain({ kind: "call", call });
  }
}

/** Why `hello`'s grafta cannot work with this panel, or nothing when it can. */
function mismatch(hello: Caller): string | undefined {
  const theirs = hello.protocol ?? 0;
  if (theirs === PROTOCOL) return undefined;
  return theirs < PROTOCOL
    ? `grafta ${hello.version} is older than this panel. Update grafta in your AI app and start it again.`
    : `This panel is older than grafta ${hello.version}. Update the panel: from Figma, or the grafta-figma-plugin.zip of grafta ${hello.version}'s release.`;
}

function connect(port: number): void {
  if (trying.has(port) || [...connections.values()].some((c) => c.port === port)) return;
  trying.add(port);
  const socket = new WebSocket(`ws://localhost:${port}`);
  const connection: Connection = { number: next++, port, socket, allowed: false };
  socket.onopen = () => {
    trying.delete(port);
    connections.set(connection.number, connection);
    render();
  };
  socket.onmessage = (event) => receive(connection, String(event.data));
  socket.onclose = () => {
    trying.delete(port);
    // A port tried with nothing on it changes nothing on screen: not drawn again.
    if (!connections.delete(connection.number)) return;
    const at = asking.indexOf(connection);
    if (at >= 0) asking.splice(at, 1);
    render();
  };
  socket.onerror = () => socket.close();
}

window.onmessage = (event: MessageEvent) => {
  const message = (event.data as { pluginMessage?: ToPanel }).pluginMessage;
  if (!message) return;
  switch (message.kind) {
    case "file":
      file = message.file;
      dev = message.selftest === true;
      edits++;
      element("selftest-section").hidden = !dev;
      break;
    case "selftest":
      element<HTMLButtonElement>("selftest").disabled = false;
      element<HTMLTextAreaElement>("selftest-table").value = message.table;
      element("selftest-table").hidden = false;
      break;
    case "known": {
      file = message.file;
      const connection = connections.get(message.connection);
      if (!connection) break;
      if (message.known) grant(connection);
      else if (!asking.includes(connection)) asking.push(connection);
      break;
    }
    case "answer": {
      const connection = connections.get(message.connection);
      if (connection) send(connection, { id: message.id, result: message.result, error: message.error, ms: message.ms });
      record({
        said: message.said ?? { kind: "change", text: message.tool, ok: message.error === undefined },
        error: message.error,
        tools: [message.tool],
        ms: message.ms,
      });
      doing(undefined);
      break;
    }
  }
  render();
};

element("allow").onclick = () => {
  const connection = asking.shift();
  if (connection?.caller) {
    toMain({ kind: "allow", caller: connection.caller });
    grant(connection);
  }
  render();
};

element("refuse").onclick = () => {
  const connection = asking.shift();
  if (connection?.caller) {
    refused.add(connection.caller.secret);
    send(connection, { allowed: false, why: "the person did not allow it" });
    connection.socket.close();
  }
  render();
};

element("clear").onclick = () => {
  last.length = 0;
  edits++;
  render();
};

element("stop").onclick = () => {
  stopped = !stopped;
  for (const connection of connections.values()) if (connection.allowed) send(connection, { stopped });
  render();
};

element("selftest").onclick = () => {
  element<HTMLButtonElement>("selftest").disabled = true;
  element<HTMLTextAreaElement>("selftest-table").value = "Running…";
  element("selftest-table").hidden = false;
  toMain({ kind: "selftest" });
};

// The corner the person drags to make the panel bigger or smaller; the size
// they leave it at is kept for next time.
const grip = element("grip");
grip.onpointerdown = (down: PointerEvent) => {
  grip.setPointerCapture(down.pointerId);
  const start = { x: down.screenX, y: down.screenY, width: window.innerWidth, height: window.innerHeight };
  const size = (event: PointerEvent, keep: boolean) =>
    toMain({ kind: "resize", width: start.width + event.screenX - start.x, height: start.height + event.screenY - start.y, keep });
  grip.onpointermove = (event) => size(event, false);
  grip.onpointerup = (event) => {
    size(event, true);
    grip.onpointermove = null;
    grip.onpointerup = null;
    grip.releasePointerCapture(event.pointerId);
  };
};

function tryAll(): void {
  for (const port of PORTS) connect(port);
}

tryAll();
setInterval(tryAll, RETRY_MS);
render();
