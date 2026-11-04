import { ArrowRight } from "../editorial/icons";
import { Cursor, FrameName, Selection } from "../figma";
import { GetGraftaButton } from "../install/Picker";
import { HeroCanvas } from "./HeroCanvas";

/** The product in five seconds: what it does, where it works, and how to get it. */
export function Hero() {
  return (
    <header
      id="hero"
      className="grid-lines relative overflow-hidden px-4 pt-20 pb-20 lg:px-0 lg:pt-28 lg:pb-24"
    >
      <FrameName className="absolute top-24 right-5 hidden lg:inline-flex">Hero</FrameName>
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-4 lg:col-start-2 lg:pr-8">
          <Selection label="Promise" size="680 × 150" className="mt-8 lg:mt-10">
            <h1 className="text-[40px] leading-[1.04] font-normal tracking-[-0.03em] sm:text-[56px] lg:text-[64px]">
              Ask your AI. <span className="text-accent-text">Get a Figma file back.</span>
            </h1>
          </Selection>
        </div>
        <div className="col-span-4 mt-16 lg:col-span-2 lg:col-start-6 lg:mt-12">
          <p className="max-w-[380px] text-[17px] leading-relaxed text-muted">
            Say what you want done to a .fig on your disk, in the AI app you already use. grafta opens it,
            makes the change, checks it, and saves a new file for you to open in Figma.
          </p>
          <div className="mt-6 flex flex-col items-start gap-3 sm:flex-row sm:items-center lg:flex-col lg:items-start xl:flex-row xl:items-center">
            <GetGraftaButton className="group inline-flex items-center justify-center gap-2.5 rounded-full bg-button py-3 pr-4 pl-5 text-[15px] font-medium whitespace-nowrap text-on-button">
              Get grafta{" "}
              <ArrowRight
                size={16}
                className="text-button-accent transition-transform group-hover:translate-x-1"
              />
            </GetGraftaButton>
            <a
              href="#how"
              className="rounded-full border border-line px-5 py-3 text-center text-[15px] font-medium whitespace-nowrap text-text no-underline"
            >
              See how it works ↓
            </a>
          </div>
        </div>
      </div>
      <Cursor name="You" className="top-[370px] left-[72%] hidden lg:block" />
      <HeroCanvas />
    </header>
  );
}
