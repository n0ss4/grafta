import { ArrowRight } from "../editorial/icons";
import { GetGraftaButton } from "../install/Picker";

/** The same uncomplicated action again, after the visitor has the answers. */
export function Install() {
  return (
    <section
      id="get"
      aria-labelledby="get-title"
      className="tone-ink overflow-hidden px-4 py-24 lg:px-0 lg:py-32"
    >
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <div className="max-w-[760px]">
            <h2
              id="get-title"
              className="text-[44px] leading-[1.04] font-normal tracking-[-0.04em] sm:text-[64px]"
            >
              Give your AI a real Figma file.
            </h2>
          </div>
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
          <p className="mt-5 text-[14px] text-muted">
            For macOS, Linux and Windows. Free and open source under the MIT licence, and not affiliated with
            Figma.
          </p>
        </div>
      </div>
    </section>
  );
}
