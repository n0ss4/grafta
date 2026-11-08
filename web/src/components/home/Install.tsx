import { Installer } from "../install/Installer";

/** Getting grafta, on the page: pick the app, do its one step, try it on the sample. */
export function Install() {
  return (
    <section
      id="get"
      aria-labelledby="get-title"
      className="tone-ink scroll-mt-16 px-4 py-20 lg:px-0 lg:py-28"
    >
      <div className="grid grid-cols-4 lg:grid-cols-8">
        <div className="col-span-4 lg:col-span-6 lg:col-start-2">
          <h2
            id="get-title"
            className="text-[44px] leading-[1.04] font-normal tracking-[-0.04em] sm:text-[56px]"
          >
            Get grafta.
          </h2>
          <p className="mt-4 max-w-[640px] text-[17px] leading-relaxed text-muted">
            Pick the app you use: one step to install it, then ask it about a Figma file.
          </p>
          <div className="mt-10">
            <Installer />
          </div>
          <p className="mt-8 text-[14px] text-muted">
            Free and open source, under the MIT licence. For macOS, Linux and Windows. Not affiliated with
            Figma.
          </p>
        </div>
      </div>
    </section>
  );
}
