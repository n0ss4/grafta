import { GetGraftaButton } from "../install/Picker";

/** Under the hero: one way to get it, and where to read how it works. */
export function GetGrafta() {
  return (
    <div className="mt-6 flex flex-wrap items-center gap-x-6 gap-y-3">
      <GetGraftaButton className="rounded-full bg-button px-5 py-2.5 text-[15px] font-medium text-on-button">
        Get grafta
      </GetGraftaButton>
      <a href="#how" className="text-[15px] font-medium text-text underline underline-offset-4">
        How it works
      </a>
      <p className="text-[14px] text-muted">
        Works in Claude, Cursor, VS Code, Codex and other AI apps that run tools on your computer.
      </p>
    </div>
  );
}
