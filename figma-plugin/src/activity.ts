// What the panel shows of the work, drawn from what the main thread sends:
// the request going on, the requests done (today's, and the file's history),
// a change waiting for the person in Ask me first, and the layers they keep
// from the AI. The main thread decides; this draws and passes clicks back.
import type { Change, LayerView, PendingView, ProtectedView, RequestView, ToMain } from "./protocol";

type Send = (message: ToMain) => void;

const live = new Map<number, RequestView>();
let past: RequestView[] = [];
/** Requests the person opened or closed; the newest is open until they close it. */
const opened = new Set<number>();
const closed = new Set<number>();
let pending: PendingView | null = null;
let now = { text: "", working: false, at: 0 };
let guarded: ProtectedView[] = [];
let copied = 0;
/** Bumped when what the list shows changes; drawn again only then. */
let version = 0;
let drawn = -1;
/** Pictures as object URLs, by request and side, revoked when replaced. */
const pictures = new Map<string, string>();

export function onRequest(request: RequestView): void {
  live.set(request.id, request);
  version++;
}
export function onHistory(requests: RequestView[]): void {
  past = requests;
  version++;
}
export function onPending(next: PendingView | null): void {
  pending = next;
}
export function onNow(text: string, working: boolean): void {
  now = { text, working, at: Date.now() };
}
export function onProtected(list: ProtectedView[]): void {
  guarded = list;
  version++;
}

export const waiting = (): boolean => pending !== null;
/** A call is running, or ended just now: the work is still in sight. */
export const working = (): boolean => now.working || Date.now() - now.at < 1500;

/** Today's requests and the file's, newest first; one from this session replaces its saved copy. */
function requests(): RequestView[] {
  const fresh = [...live.values()];
  const kept = past.filter((p) => !fresh.some((r) => r.started === p.started));
  return [...fresh, ...kept].sort((a, b) => b.started - a.started);
}

const el = <T extends HTMLElement>(tag: string, className?: string, text?: string): T => {
  const node = document.createElement(tag) as T;
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

const time = (ms: number): string => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
const today = (ms: number): boolean => new Date(ms).toDateString() === new Date().toDateString();

function url(key: string, bytes: Uint8Array | undefined): string | undefined {
  if (!bytes) return undefined;
  const old = pictures.get(key);
  if (old) URL.revokeObjectURL(old);
  const made = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
  pictures.set(key, made);
  return made;
}

/** What a request did, in a few words: "Promo card made", "Card changed". */
function title(r: RequestView): string {
  const first = r.layers.find((l) => l.made && !l.removed) ?? r.layers.find((l) => !l.removed) ?? r.layers[0];
  if (!first) return "Nothing changed";
  const what = first.made ? "made" : first.removed ? "deleted" : "changed";
  return `${first.name} ${what}`;
}

function counts(r: RequestView): string {
  const parts = [
    r.made ? `${r.made} ${r.made === 1 ? "layer" : "layers"} made` : "",
    r.changed ? `${r.changed} changed` : "",
    r.removed ? `${r.removed} deleted` : "",
  ].filter(Boolean);
  return parts.join(" · ") || "no change";
}

function value(text: string, className: string): HTMLElement {
  const node = el<HTMLSpanElement>("span", `value ${className}`, text);
  node.title = text;
  if (/^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(text)) {
    const swatch = el<HTMLElement>("i", "swatch");
    swatch.style.background = text;
    node.prepend(swatch);
  }
  return node;
}

function diff(changes: Change[], more: number): HTMLElement {
  const grid = el<HTMLDivElement>("div", "diff");
  for (const change of changes) {
    const line = el<HTMLDivElement>("div", "change");
    const values = el<HTMLSpanElement>("span", "values");
    values.append(value(change.before, "v-was"), el("span", "arrow", "→"), value(change.after, "v-now"));
    const property = el<HTMLSpanElement>("span", "property", change.property);
    property.title = change.property;
    line.append(property, values);
    grid.append(line);
  }
  if (more) grid.append(el("span", "and", `and ${more} more`));
  return grid;
}

function figure(src: string | undefined, label: string, className: string, empty: string): HTMLElement {
  const frame = el<HTMLElement>("figure", `thumb ${className}`);
  frame.append(el("figcaption", "", label));
  if (src) {
    const image = el<HTMLImageElement>("img");
    image.src = src;
    image.alt = label;
    frame.append(image);
  } else frame.append(el("span", "nothing", empty));
  return frame;
}

function layerRow(layer: LayerView, live: boolean, send: Send): HTMLElement {
  const row = el<HTMLDivElement>("div", "layer");
  const name = el<HTMLButtonElement>("button", "layer-name", layer.name);
  name.title = layer.removed ? "This layer was deleted" : "Show it on the canvas";
  name.disabled = layer.removed || !live;
  name.onclick = () => send({ kind: "focus", id: layer.id });
  row.append(name);
  if (layer.made) row.append(el("span", "tag made", "new"));
  if (layer.removed) row.append(el("span", "tag removed", "deleted"));
  if (layer.changes.length) row.append(diff(layer.changes, layer.more));
  return row;
}

function card(r: RequestView, send: Send, draw: () => void): HTMLElement {
  const box = el<HTMLDivElement>("div", `req open${r.reverted ? " reverted" : ""}`);
  const head = el<HTMLButtonElement>("button", "req-head");
  head.append(el("span", "when", r.live && today(r.started) && r === requests()[0] ? `This request · ${time(r.started)}` : time(r.started)), el("span", "chevron"));
  head.onclick = () => {
    opened.delete(r.id);
    closed.add(r.id);
    version++;
    draw();
  };
  box.append(head, el("div", "req-sub", `${r.reverted ? "Reverted · " : ""}${counts(r)}${r.context ? ` · in “${r.context}”` : ""}`));
  if (r.before || r.after) {
    const pair = el<HTMLDivElement>("div", "thumbs");
    pair.append(
      figure(url(`${r.id}:before`, r.before), "Before", "before", "Not there yet"),
      el("span", "arrow", "→"),
      figure(url(`${r.id}:after`, r.after), r.reverted ? "Now (reverted)" : "After", "after", "Gone"),
    );
    box.append(pair);
  }
  const layers = el<HTMLDivElement>("div", "layers");
  for (const layer of r.layers) layers.append(layerRow(layer, r.live, send));
  box.append(layers);
  for (const note of r.notes) box.append(el("div", "note", note));
  if (r.live) {
    const actions = el<HTMLDivElement>("div", "req-actions");
    const showAll = el<HTMLButtonElement>("button", "grow", "Show all on canvas");
    showAll.onclick = () => send({ kind: "show", request: r.id });
    const back = el<HTMLButtonElement>("button", "grow danger", r.reverted ? "Reverted" : "↺ Revert this request");
    back.disabled = !r.revertable;
    back.title = r.revertable ? "Put every layer back as it was before this request, in one step of undo" : r.reverted ? "Already reverted" : "Nothing left to put back";
    back.onclick = () => send({ kind: "revert", request: r.id });
    actions.append(showAll, back);
    box.append(actions);
  }
  return box;
}

function row(r: RequestView, draw: () => void): HTMLElement {
  const line = el<HTMLButtonElement>("button", `req closed${r.reverted ? " reverted" : ""}`);
  line.append(el("span", "when", time(r.started)));
  if (r.reverted) line.append(el("span", "reverted-tag", "↺ Reverted"));
  const changes = r.layers.reduce((n, l) => n + Math.max(1, l.changes.length + l.more), 0);
  line.append(el("span", "what", `${title(r)} · ${changes} ${changes === 1 ? "change" : "changes"}`), el("span", "chevron"));
  line.onclick = () => {
    closed.delete(r.id);
    opened.add(r.id);
    version++;
    draw();
  };
  return line;
}

/** A request's changes as text for the team: Slack, a ticket, a review. */
function changelog(file: string, all: RequestView[]): string {
  const lines = [`Changes to “${file}” through grafta, ${new Date().toLocaleDateString()}`];
  for (const r of [...all].reverse()) {
    lines.push(`- ${time(r.started)} ${r.reverted ? "(reverted) " : ""}${title(r)}: ${counts(r)}`);
    for (const l of r.layers) {
      const what = l.made ? "new" : l.removed ? "deleted" : l.changes.map((c) => `${c.property} ${c.before} → ${c.after}`).join("; ");
      lines.push(`  - ${l.name}: ${what}`);
    }
  }
  return lines.join("\n");
}

async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Figma's panel may not grant the clipboard: the old way still works there.
    const area = el<HTMLTextAreaElement>("textarea");
    area.value = text;
    document.body.append(area);
    area.select();
    const done = document.execCommand("copy");
    area.remove();
    return done;
  }
}

/** Draw everything in its place in ui.html. */
export function draw(send: Send, file: string, again: () => void): void {
  // The change waiting for the person.
  const waitingBox = document.getElementById("pending")!;
  waitingBox.hidden = !pending;
  const left = pending ? Math.max(0, Math.ceil((pending.until - Date.now()) / 1000)) : 0;
  const countdown = `Nothing changes in the file until you choose · ${left} s`;
  // The card is built once per change: drawn again each second, a click
  // between press and release would land on a button no longer there.
  if (pending && waitingBox.dataset.id === String(pending.id)) {
    const sub = waitingBox.querySelector(".pending-sub");
    if (sub) sub.textContent = countdown;
  } else if (pending) {
    const p = pending;
    waitingBox.dataset.id = String(p.id);
    waitingBox.replaceChildren();
    waitingBox.append(el("div", "pending-title", `${p.caller} wants to ${p.text}`), el("div", "pending-sub", countdown));
    if (p.now) waitingBox.append(figure(url(`pending:${p.id}`, p.now), "Now", "before wide", ""));
    if (p.planned.length) waitingBox.append(diff(p.planned, 0));
    const choose = el<HTMLDivElement>("div", "req-actions");
    const apply = el<HTMLButtonElement>("button", "primary grow", "✓ Apply");
    const decide = (apply: boolean, rest: boolean) => {
      for (const button of waitingBox.querySelectorAll("button")) (button as HTMLButtonElement).disabled = true;
      send({ kind: "decide", id: p.id, apply, rest });
    };
    apply.onclick = () => decide(true, false);
    const skip = el<HTMLButtonElement>("button", "grow", "Skip");
    skip.onclick = () => decide(false, false);
    choose.append(apply, skip);
    const rest = el<HTMLButtonElement>("button", "link rest", "Apply the rest of this request without asking");
    rest.onclick = () => decide(true, true);
    waitingBox.append(choose, rest);
  } else delete waitingBox.dataset.id;

  // The work going on.
  const nowBox = document.getElementById("now")!;
  const newest = requests()[0];
  nowBox.hidden = !working() || !!pending;
  if (!nowBox.hidden) {
    const done = newest?.live && Date.now() - newest.ended < 8000 ? newest.layers.length : 0;
    document.getElementById("now-title")!.textContent = done ? `Working · ${done} ${done === 1 ? "layer" : "layers"} so far` : "Working";
    document.getElementById("now-text")!.textContent = now.text;
  }

  // The requests done, and the layers kept: drawn again only when they change,
  // so a click is never lost to a redraw.
  if (drawn === version) return;
  drawn = version;
  const all = requests();
  const todays = all.filter((r) => today(r.started));
  const changes = todays.reduce((n, r) => n + r.layers.length, 0);
  document.getElementById("requests-count")!.textContent = todays.length
    ? `Today · ${todays.length} ${todays.length === 1 ? "request" : "requests"} · ${changes} ${changes === 1 ? "layer" : "layers"}`
    : "Activity";
  const list = document.getElementById("requests")!;
  list.replaceChildren(...all.map((r, i) => ((i === 0 && !closed.has(r.id)) || opened.has(r.id) ? card(r, send, again) : row(r, again))));
  document.getElementById("requests-empty")!.hidden = all.length > 0;
  const copyButton = document.getElementById("copy") as HTMLButtonElement;
  copyButton.hidden = todays.length === 0;
  copyButton.textContent = Date.now() - copied < 1500 ? "Copied" : "Copy for the team";
  copyButton.onclick = () =>
    void copy(changelog(file, todays)).then((done) => {
      if (done) copied = Date.now();
      version++;
      again();
      setTimeout(() => {
        version++;
        again();
      }, 1600);
    });

  // The layers kept from the AI.
  const kept = document.getElementById("protected-list")!;
  kept.replaceChildren(
    ...guarded.map((p) => {
      const item = el<HTMLLIElement>("li", "kept");
      item.append(el("span", "kept-type", p.type.charAt(0) + p.type.slice(1).toLowerCase().replace("_", " ")), el("span", "kept-name", `“${p.name}”`));
      const remove = el<HTMLButtonElement>("button", "link", "Unprotect");
      remove.onclick = () => send({ kind: "unprotect", id: p.id });
      item.append(remove);
      return item;
    }),
  );
  kept.hidden = guarded.length === 0;
}
