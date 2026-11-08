import { GetGraftaButton } from "../install/Picker";

/** Beside the line: one way to get it, and what the video below is. */
export function GetGrafta() {
  return (
    <div className="mt-4 flex flex-wrap items-center gap-x-5 gap-y-3">
      <GetGraftaButton className="rounded-full bg-button px-5 py-2.5 text-[15px] font-medium text-on-button">
        Get grafta
      </GetGraftaButton>
      <p className="max-w-[300px] text-[13px] leading-snug text-muted">
        The video: real sessions in Claude Code and Claude Desktop, shortened. The windows are drawn.
      </p>
    </div>
  );
}
