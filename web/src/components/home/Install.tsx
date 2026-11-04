import { ArrowRight } from "../editorial/icons";
import { Mark } from "../editorial/Mark";
import { GetGraftaButton } from "../install/Picker";
import { FrameName, Selection } from "../figma";

/** The same uncomplicated action again, after the visitor has the answers. */
export function Install() {
  return (
    <section
      id="get"
      aria-labelledby="get-title"
      className="tone-ink grid-lines overflow-hidden px-4 pt-24 pb-0 lg:px-0 lg:pt-32"
    >
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <FrameName>Get grafta</FrameName>
          <Selection label="Install" className="mt-8 max-w-[760px]">
            <h2
              id="get-title"
              className="text-[44px] leading-[1.04] font-normal tracking-[-0.04em] sm:text-[64px]"
            >
              Give your AI a real Figma file.
            </h2>
          </Selection>
          <p className="mt-12 max-w-[520px] text-[17px] leading-relaxed text-muted">
            Pick the app you already use. You will get one download, command or configuration to copy.
          </p>
          <GetGraftaButton className="group mt-7 inline-flex items-center gap-2.5 rounded-full bg-button py-3 pr-4 pl-5 text-[15px] font-medium text-on-button">
            Get grafta{" "}
            <ArrowRight
              size={16}
              className="text-button-accent transition-transform group-hover:translate-x-1"
            />
          </GetGraftaButton>
          <p className="mt-5 font-mono text-[11px] text-muted">
            macOS · Linux · Windows · MIT · not affiliated with Figma
          </p>
        </div>
      </div>
      <div className="mt-20 flex items-center gap-5 border-t border-white/10 px-4 py-8 lg:px-[12.5%]">
        <Mark size={34} />
        <span className="font-mono text-[10px] tracking-[0.08em] text-muted uppercase">
          local files · exact edits · open source
        </span>
      </div>
    </section>
  );
}
