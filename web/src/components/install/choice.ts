import { useSyncExternalStore } from "react";
import { clients } from "~/content/clients";

// The way in a visitor chose, shared by the install section and the dialog, and
// kept for the visit. Without storage (private windows, blocked site data) it
// still works, for the page's life.
const KEY = "grafta:way-in";
const first = clients[0]?.id ?? "claude-desktop";
const known = (id: string | null): id is string => clients.some((client) => client.id === id);

let chosen: string | undefined;
const listeners = new Set<() => void>();

function read(): string {
  if (chosen === undefined) {
    let stored: string | null = null;
    try {
      stored = window.sessionStorage.getItem(KEY);
    } catch {
      stored = null;
    }
    chosen = known(stored) ? stored : first;
  }
  return chosen;
}

export function choose(id: string): void {
  if (!known(id)) return;
  chosen = id;
  try {
    window.sessionStorage.setItem(KEY, id);
  } catch {
    // kept for the page's life only
  }
  listeners.forEach((listener) => {
    listener();
  });
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The chosen way in's id; the first on the server, so the page renders it. */
export function useChoice(): string {
  return useSyncExternalStore(subscribe, read, () => first);
}

/** For tests: forget the choice. */
export function resetChoice(): void {
  chosen = undefined;
}
