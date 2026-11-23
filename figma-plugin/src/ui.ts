// grafta's panel, its window: the connections to grafta on this computer, one
// to each grafta that answers on its ports, and what the person sees and
// decides. It sends nothing anywhere but to those connections.
import { PROTOCOL } from "./contract";
import { PORTS } from "./ports";
import * as activity from "./activity";
import type { Caller, Call, FileInfo, ToMain, ToPanel } from "./protocol";

/** How often a port with nothing on it is tried again. */
const RETRY_MS = 1000;

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
/** Whether each change is selected and brought into view on the canvas. */
let follow = true;

/** Whether each change waits for the person, Ask me first. */
let ask = false;

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
  element("stop-text").textContent = stopped ? "Resume" : "Stop";
  element("follow").setAttribute("aria-pressed", String(follow));
  element("mode-apply").setAttribute("aria-pressed", String(!ask));
  element("mode-ask").setAttribute("aria-pressed", String(ask));
  // Connected, the pill says what goes on: working, or waiting for the person.
  if (state === "connected") {
    const app = allowed.length === 1 ? (allowed[0].caller?.name ?? "An app") : `${allowed.length} apps`;
    const doing = activity.waiting() ? "waiting for you" : activity.working() ? "working" : "connected";
    element("pill").textContent = `${app} · ${doing}`;
    document.body.dataset.doing = activity.waiting() ? "waiting" : activity.working() ? "working" : "idle";
  } else delete document.body.dataset.doing;
  // The state in full only when it needs saying: connected, the work speaks.
  element("hero").hidden = state === "connected";
  element("work").hidden = state !== "connected" && state !== "stopped";
  element("foot").hidden = state !== "connected" && state !== "stopped";
  activity.draw(toMain, file.name, render);
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
    const call: Call = {
      connection: connection.number,
      caller: connection.caller?.name,
      id: message.id,
      tool: message.tool,
      args: (message.args ?? {}) as Record<string, unknown>,
    };
    toMain({ kind: "call", call });
  }
}

/** Why `hello`'s grafta cannot work with this panel, or nothing when it can. */
function mismatch(hello: Caller): string | undefined {
  const theirs = hello.protocol ?? 0;
  if (theirs === PROTOCOL) return undefined;
  return theirs < PROTOCOL
    ? `grafta ${hello.version} is older than this panel. Update grafta in your AI app and start it again.`
    : `This panel is older than grafta ${hello.version}. Import the panel again from grafta-figma-plugin.zip of grafta ${hello.version}'s release.`;
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
      element("selftest-section").hidden = message.selftest !== true;
      break;
    case "selftest":
      element<HTMLButtonElement>("selftest").disabled = false;
      element<HTMLTextAreaElement>("selftest-table").value = message.table;
      element("selftest-table").hidden = false;
      break;
    case "follow":
      follow = message.on;
      break;
    case "ask":
      ask = message.on;
      break;
    case "now":
      activity.onNow(message.text, message.working);
      break;
    case "request":
      activity.onRequest(message.request);
      break;
    case "history":
      activity.onHistory(message.requests);
      break;
    case "pending": {
      activity.onPending(message.pending);
      // grafta waits longer for a call the person is deciding on.
      const asked = message.pending && connections.get(message.pending.connection);
      if (asked && message.pending) send(asked, { asking: message.pending.call });
      break;
    }
    case "protected":
      activity.onProtected(message.list);
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

element("follow").onclick = () => {
  follow = !follow;
  toMain({ kind: "follow", on: follow });
  render();
};

element("mode-apply").onclick = () => {
  ask = false;
  toMain({ kind: "ask", on: false });
  render();
};

element("mode-ask").onclick = () => {
  ask = true;
  toMain({ kind: "ask", on: true });
  render();
};

element("protect").onclick = () => toMain({ kind: "protect" });

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
// A countdown, and the work fading from sight once done.
setInterval(() => {
  if (activity.waiting() || document.body.dataset.doing === "working") render();
}, 500);
render();
