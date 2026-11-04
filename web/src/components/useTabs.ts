import { useId, useRef, type KeyboardEvent } from "react";

/**
 * The WAI-ARIA tabs pattern, as props to spread: one tab in the tab order at a
 * time, arrows, Home and End moving between tabs and selecting, each panel
 * labelled by its tab. The caller lays the tablist and panels out where it likes.
 */
export function useTabs(ids: string[], value: string, onValueChange: (id: string) => void) {
  const base = useId();
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const tabId = (id: string) => `${base}-tab-${id}`;
  const panelId = (id: string) => `${base}-panel-${id}`;

  function onKeyDown(event: KeyboardEvent, index: number) {
    const last = ids.length - 1;
    const moves: Record<string, number> = {
      ArrowRight: index === last ? 0 : index + 1,
      ArrowLeft: index === 0 ? last : index - 1,
      Home: 0,
      End: last,
    };
    const next = moves[event.key];
    const id = next === undefined ? undefined : ids[next];
    if (next === undefined || id === undefined) return;
    event.preventDefault();
    onValueChange(id);
    tabs.current[next]?.focus();
  }

  return {
    list: (label: string) => ({ role: "tablist" as const, "aria-label": label }),
    tab: (id: string, index: number) => {
      const selected = id === value;
      return {
        type: "button" as const,
        role: "tab" as const,
        id: tabId(id),
        "aria-selected": selected,
        "aria-controls": panelId(id),
        tabIndex: selected ? 0 : -1,
        "data-state": selected ? "active" : "inactive",
        ref: (node: HTMLButtonElement | null) => {
          tabs.current[index] = node;
        },
        onClick: () => {
          onValueChange(id);
        },
        onKeyDown: (event: KeyboardEvent) => {
          onKeyDown(event, index);
        },
      };
    },
    panel: (id: string) => ({
      role: "tabpanel" as const,
      id: panelId(id),
      "aria-labelledby": tabId(id),
      hidden: id !== value,
      tabIndex: 0,
    }),
  };
}
