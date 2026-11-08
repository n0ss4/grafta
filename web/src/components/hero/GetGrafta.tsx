import { GetGraftaButton } from "../install/Picker";

/** Under the hero: one way to get it, and the questions people ask. */
export function GetGrafta() {
  return (
    <div className="mt-5 flex flex-wrap items-center gap-x-6 gap-y-3">
      <GetGraftaButton className="rounded-full bg-button px-5 py-2.5 text-[15px] font-medium text-on-button">
        Get grafta
      </GetGraftaButton>
      <a href="#faq" className="text-[15px] font-medium text-text underline underline-offset-4">
        Questions
      </a>
    </div>
  );
}
