import { useEffect, useRef, useState } from "react";
import { clients } from "~/content/clients";
import { cn } from "~/lib/cn";
import { CopyButton } from "./CopyButton";
import { ClientMark } from "./ClientMark";

export const openPicker = () => window.dispatchEvent(new Event("grafta:picker"));

export function GetGraftaButton({
  className,
  children = "Get grafta",
}: {
  className?: string;
  children?: React.ReactNode;
}) {
  return (
    <button type="button" onClick={openPicker} className={cn("cursor-pointer", className)}>
      {children}
    </button>
  );
}

/** One compact route from any CTA to the exact line or file a person's app needs. */
export function Picker() {
  const ref = useRef<HTMLDialogElement>(null);
  const [picked, setPicked] = useState(clients[0]?.id ?? "claude-code");
  const client = clients.find((item) => item.id === picked) ?? clients[0];

  useEffect(() => {
    const open = () => {
      ref.current?.showModal();
    };
    window.addEventListener("grafta:picker", open);
    return () => {
      window.removeEventListener("grafta:picker", open);
    };
  }, []);

  if (!client) return null;
  return (
    <dialog
      ref={ref}
      aria-labelledby="picker-title"
      className="m-auto w-[min(480px,calc(100%-2rem))] rounded-2xl border border-white/15 bg-ink p-0 text-paper shadow-2xl backdrop:bg-ink/55"
    >
      <div className="p-6">
        <div className="flex items-start justify-between gap-6">
          <div>
            <h2 id="picker-title" className="text-[24px] font-normal tracking-[-0.02em]">
              Get grafta
            </h2>
            <p className="mt-1 text-[14px] text-[#9DA3AB]">Which app do you use?</p>
          </div>
          <button
            type="button"
            onClick={() => {
              ref.current?.close();
            }}
            className="cursor-pointer text-[22px] text-[#9DA3AB]"
            aria-label="Close"
          >
            ×
          </button>
        </div>
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3">
          {clients.map((item) => (
            <button
              type="button"
              key={item.id}
              onClick={() => {
                setPicked(item.id);
              }}
              aria-pressed={item.id === picked}
              className="flex cursor-pointer items-center gap-2.5 rounded-lg border border-white/15 px-3 py-2 text-left text-[13px] text-[#D6D9DE] hover:border-white/40 aria-pressed:border-bud aria-pressed:bg-white/10 aria-pressed:text-white"
            >
              <ClientMark id={item.id} size={16} />
              {item.name}
            </button>
          ))}
        </div>
        <div className="mt-4 rounded-[10px] border border-white/15 bg-white/[0.06] p-3.5">
          {client.file && <p className="mb-2 font-mono text-[11px] text-[#9DA3AB]">{client.file}</p>}
          <code className="block overflow-x-auto font-mono text-[12px] leading-5 whitespace-pre text-paper">
            {client.snippet}
          </code>
        </div>
        <div className="mt-4 flex items-center justify-between gap-3">
          <p className="text-[12px] leading-5 text-[#9DA3AB]">{client.note}</p>
          {client.kind === "download" ? (
            <a
              href={client.snippet}
              className="flex-none rounded-lg bg-bud px-3 py-1.5 text-[14px] font-semibold text-ink no-underline"
            >
              Download
            </a>
          ) : (
            <CopyButton text={client.snippet} label="Copy line" tone="primary" />
          )}
        </div>
      </div>
    </dialog>
  );
}
