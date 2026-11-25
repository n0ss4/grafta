import { useSyncExternalStore } from "react";
import { Moon, Sun } from "./icons";

type Theme = "light" | "dark";

// The browser's bar takes the page's background, in either mode.
const BAR: Record<Theme, string> = { light: "#FAFAFB", dark: "#060607" };

function subscribe(onChange: () => void) {
  window.addEventListener("themechange", onChange);
  return () => {
    window.removeEventListener("themechange", onChange);
  };
}
function current(): Theme {
  return document.documentElement.dataset.theme === "light" ? "light" : "dark";
}

function pick(next: Theme) {
  document.documentElement.dataset.theme = next;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", BAR[next]);
  try {
    sessionStorage.setItem("theme", next);
  } catch {
    // kept for the page's life only
  }
  window.dispatchEvent(new Event("themechange"));
}

const choices = [
  { theme: "light", label: "Light", Icon: Sun },
  { theme: "dark", label: "Dark", Icon: Moon },
] as const;

/**
 * Light or Ink, both in sight, the one showing pressed: Ink until the visitor
 * picks; the pick is kept for the visit (root.tsx applies it before paint).
 */
export function ThemeToggle() {
  const theme = useSyncExternalStore<Theme | null>(subscribe, current, () => null);
  return (
    <div
      role="group"
      aria-label="Colour mode"
      className="inline-flex gap-0.5 rounded-full p-[3px] ring-1 ring-line ring-inset"
    >
      {choices.map(({ theme: value, label, Icon }) => (
        <button
          key={value}
          type="button"
          onClick={() => {
            pick(value);
          }}
          aria-pressed={theme === value}
          aria-label={label}
          title={label}
          className="grid h-[26px] w-7 cursor-pointer place-items-center rounded-full text-faint transition-colors hover:text-text aria-pressed:bg-raised aria-pressed:text-text aria-pressed:ring-1 aria-pressed:ring-line aria-pressed:ring-inset"
        >
          <Icon size={14} strokeWidth={2} />
        </button>
      ))}
    </div>
  );
}
