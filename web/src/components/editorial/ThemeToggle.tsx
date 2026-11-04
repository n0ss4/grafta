import { useSyncExternalStore } from "react";
import { Moon, Sun } from "./icons";

type Theme = "light" | "dark";
const query = "(prefers-color-scheme: dark)";

function subscribe(onChange: () => void) {
  const media = window.matchMedia(query);
  media.addEventListener("change", onChange);
  window.addEventListener("themechange", onChange);
  return () => {
    media.removeEventListener("change", onChange);
    window.removeEventListener("themechange", onChange);
  };
}
function current(): Theme {
  const set = document.documentElement.dataset.theme;
  if (set === "dark" || set === "light") return set;
  return window.matchMedia(query).matches ? "dark" : "light";
}

/** Paper or Ink, whatever the system says; the choice is kept for the visit (root.tsx applies it before paint). */
export function ThemeToggle() {
  const theme = useSyncExternalStore<Theme | null>(subscribe, current, () => null);
  function toggle() {
    const next: Theme = current() === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    sessionStorage.setItem("theme", next);
    window.dispatchEvent(new Event("themechange"));
  }
  const dark = theme === "dark";
  const icon =
    "absolute inset-0 m-auto motion-safe:transition-[transform,opacity] motion-safe:duration-500 motion-safe:ease-[cubic-bezier(.2,.8,.2,1)]";
  return (
    <button
      type="button"
      onClick={toggle}
      aria-pressed={dark}
      aria-label="Dark mode"
      title={dark ? "Paper" : "Ink"}
      className="group relative grid size-9 flex-none cursor-pointer place-items-center rounded-full border border-line text-text transition-colors hover:border-text hover:bg-text hover:text-bg"
    >
      <Moon
        size={16}
        className={`${icon} ${dark ? "scale-50 rotate-90 opacity-0" : "scale-100 rotate-0 opacity-100"}`}
      />
      <Sun
        size={16}
        className={`${icon} ${dark ? "scale-100 rotate-0 opacity-100" : "scale-50 -rotate-90 opacity-0"}`}
      />
    </button>
  );
}
