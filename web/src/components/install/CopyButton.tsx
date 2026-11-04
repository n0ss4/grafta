import { useEffect, useRef, useState } from "react";
import { copyText } from "~/lib/copy";
import { cn } from "~/lib/cn";

interface Props {
  text: string;
  label?: string;
  /** "primary" is the filled button, for the one action of a panel. */
  tone?: "quiet" | "primary";
}

/** Copies `text`; says "Copied", or "Select it" when the browser refuses, through a polite live region. */
export function CopyButton({ text, label = "Copy", tone = "quiet" }: Props) {
  const [state, setState] = useState<"idle" | "copied" | "refused">("idle");
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(
    () => () => {
      clearTimeout(timer.current);
    },
    [],
  );

  async function onClick() {
    setState((await copyText(text)) ? "copied" : "refused");
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      setState("idle");
    }, 1600);
  }

  const shown = state === "copied" ? "Copied" : state === "refused" ? "Select it" : label;
  return (
    <button
      type="button"
      onClick={() => void onClick()}
      className={cn(
        "flex-none cursor-pointer rounded-lg border px-3 py-1.5 text-[14px] font-semibold",
        tone === "primary"
          ? "border-transparent bg-button text-on-button hover:opacity-90"
          : "border-line text-text hover:border-text",
      )}
    >
      <span aria-live="polite">{shown}</span>
    </button>
  );
}
