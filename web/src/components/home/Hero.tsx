import { Client } from "../client/Client";
import type { ClientId, Recording } from "../client/types";
import { ArrowRight } from "../editorial/icons";

/**
 * 00: the promise, and a person's request played in their AI client beside it:
 * who asks, where, and the Figma file that comes back,
 * before any scroll. As the approved design draws it.
 */
export function Hero({
  first,
  client,
  onClient,
}: {
  first: Recording;
  client: string;
  onClient: (client: ClientId) => void;
}) {
  return (
    <header id="client" className="px-4 pt-10 pb-16 sm:pt-14 lg:px-[8.333%] lg:pt-16 lg:pb-20">
      <div className="grid gap-y-6 lg:grid-cols-[minmax(0,760fr)_minmax(0,380fr)] lg:gap-x-[60px]">
        <div>
          <p className="micro text-muted">00 — An MCP server for Figma files</p>
          <h1 className="mt-6 text-[36px] leading-[1.04] font-normal tracking-[-0.03em] min-[360px]:text-[40px] sm:text-[64px]">
            Ask your agent. <span className="text-accent-text">Get a Figma file back.</span>
          </h1>
        </div>
        <div className="lg:pt-8">
          <p className="text-[17px] leading-relaxed text-muted">
            Say what you want done to a .fig on your disk. grafta opens it, makes the change, reads it back,
            and saves a new file for you to open in Figma. No account, no API key, no cloud.
          </p>
          <a
            href="#install"
            className="group mt-6 inline-flex items-center gap-2.5 rounded-full bg-button py-3 pr-4 pl-5 text-[15px] font-medium text-on-button no-underline"
          >
            Install grafta
            <ArrowRight
              size={16}
              className="text-button-accent transition-transform group-hover:translate-x-1"
            />
          </a>
        </div>
      </div>
      <div className="mt-10 lg:mt-12">
        <Client first={first} client={client} onClient={onClient} />
      </div>
    </header>
  );
}
