// grafta's panel, its window: the connections to grafta on this computer, one
// to each grafta that answers on its ports, and what the person sees and
// decides. It sends nothing anywhere but to those connections.
import { PORTS } from "./ports";
import type { Caller, Call, FileInfo, ToMain, ToPanel } from "./protocol";

/** How often a port with nothing on it is tried again. */
const RETRY_MS = 1000;
/** How many calls done are shown. */
const LAST = 5;

interface Connection {
  number: number;
  port: number;
  socket: WebSocket;
  caller?: Caller;
  allowed: boolean;
}

const connections = new Map<number, Connection>();
const trying = new Set<number>();
/** Callers refused in this session, by secret: not asked about again. */
const refused = new Set<string>();
/** Connections waiting for the person to allow or refuse them. */
const asking: Connection[] = [];
const last: string[] = [];
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

function render(): void {
  const open = [...connections.values()];
  const allowed = open.filter((c) => c.allowed);
  const dot = element<HTMLSpanElement>("dot");
  dot.className = stopped ? "dot stopped" : allowed.length ? "dot on" : "dot";
  element("state").textContent = stopped
    ? "Stopped: grafta can do nothing here"
    : allowed.length
      ? `Connected to ${allowed.length === 1 ? allowed[0].caller?.name : `${allowed.length} apps`}`
      : open.length
        ? "grafta found on this computer"
        : "Looking for grafta on this computer…";
  const list = element<HTMLUListElement>("callers");
  list.replaceChildren(...allowed.map((c) => item(`${c.caller?.name ?? "an app"} · grafta ${c.caller?.version ?? ""}`)));
  element("callers-section").hidden = allowed.length === 0;
  const waiting = asking[0];
  element("ask").hidden = !waiting;
  if (waiting) element("ask-name").textContent = waiting.caller?.name ?? "An app";
  element<HTMLUListElement>("last").replaceChildren(...last.map(item));
  element("stop").textContent = stopped ? "Resume" : "Stop";
}

function item(text: string): HTMLLIElement {
  const li = document.createElement("li");
  li.textContent = text;
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
    element("doing").textContent = `Doing: ${message.tool}`;
    const call: Call = { connection: connection.number, id: message.id, tool: message.tool, args: (message.args ?? {}) as Record<string, unknown> };
    toMain({ kind: "call", call });
  }
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
    connections.delete(connection.number);
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
      last.unshift(`${message.error === undefined ? "✓" : "✕"} ${message.tool} · ${message.ms} ms`);
      last.length = Math.min(last.length, LAST);
      element("doing").textContent = "Done.";
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

element("stop").onclick = () => {
  stopped = !stopped;
  for (const connection of connections.values()) if (connection.allowed) send(connection, { stopped });
  render();
};

function tryAll(): void {
  for (const port of PORTS) connect(port);
}

tryAll();
setInterval(tryAll, RETRY_MS);
render();
