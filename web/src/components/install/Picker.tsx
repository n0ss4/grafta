import { useEffect, useRef } from "react";
import { cn } from "~/lib/cn";
import { Installer } from "./Installer";

export const openPicker = () => window.dispatchEvent(new Event("grafta:picker"));

/**
 * Every "Get grafta" away from the install section: a link to it, which opens
 * the same installer in a dialog once scripts run, so the visitor stays where
 * they are.
 */
export function GetGraftaButton({
  className,
  children = "Get grafta",
}: {
  className?: string;
  children?: React.ReactNode;
}) {
  return (
    <a
      href="#get"
      aria-haspopup="dialog"
      onClick={(event) => {
        event.preventDefault();
        openPicker();
      }}
      className={cn("cursor-pointer no-underline", className)}
    >
      {children}
    </a>
  );
}

/** The installer in a dialog, the same as the section's (Installer.tsx). */
export function Picker() {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const open = () => {
      ref.current?.showModal();
    };
    window.addEventListener("grafta:picker", open);
    return () => {
      window.removeEventListener("grafta:picker", open);
    };
  }, []);

  return (
    <dialog
      ref={ref}
      aria-labelledby="picker-title"
      className="m-auto max-h-[calc(100svh-2rem)] w-[min(640px,calc(100%-2rem))] overflow-y-auto rounded-2xl border border-line bg-sunk p-0 text-text shadow-2xl backdrop:bg-ink/55"
    >
      <div className="p-6">
        <div className="flex items-start justify-between gap-6">
          <h2 id="picker-title" className="text-[26px] font-normal tracking-[-0.02em]">
            Get grafta
          </h2>
          <button
            type="button"
            onClick={() => {
              ref.current?.close();
            }}
            className="-mt-1 cursor-pointer text-[24px] leading-none text-muted hover:text-text"
            aria-label="Close"
          >
            ×
          </button>
        </div>
        <div className="mt-5">
          <Installer compact />
        </div>
      </div>
    </dialog>
  );
}
