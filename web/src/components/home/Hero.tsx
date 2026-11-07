import { ArrowRight } from "../editorial/icons";
import { GetGraftaButton } from "../install/Picker";

/** The product in five seconds: what it does, where it works, and how to get it. */
export function Hero() {
  return (
    <header id="hero" className="relative overflow-hidden px-4 pt-20 pb-20 lg:px-0 lg:pt-28 lg:pb-24">
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-4 lg:col-start-2 lg:pr-8">
          <div className="mt-8 lg:mt-10">
            <h1 className="text-[40px] leading-[1.04] font-normal tracking-[-0.03em] sm:text-[56px] lg:text-[64px]">
              Talk to <span className="text-accent-text">your Figma files.</span>
            </h1>
          </div>
        </div>
        <div className="col-span-4 mt-16 lg:col-span-2 lg:col-start-6 lg:mt-12">
          <p className="max-w-[380px] text-[17px] leading-relaxed text-muted">
            Get a Figma file, and talk to it. Ask your AI what's in it, change it, or turn it into code, in
            whatever language you work in. No Figma account, no Figma app running, no API limits.
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
      <picture>
        <source
          media="(max-width: 700px)"
          srcSet={`${import.meta.env.BASE_URL}showcase/talk-phone.webp`}
          width={780}
          height={2522}
        />
        <img
          src={`${import.meta.env.BASE_URL}showcase/talk.webp`}
          alt="Three requests to an AI about one Figma file. What's in this file: it answers an order tracking card with 9 colour variables. Make it light and check the contrast: the card in its light theme, 9 of 9 texts passing WCAG AA, saved as a new file. Now turn it into code: CSS, and the same card drawn from it."
          width={1320}
          height={676}
          className="relative z-10 mx-auto mt-12 block h-auto w-full max-w-[1120px] rounded-3xl border border-line lg:mt-16"
        />
      </picture>
    </header>
  );
}
