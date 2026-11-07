import { useSyncExternalStore } from "react";
import type { Talk } from "~/content/talk";
import { useTabs } from "../useTabs";
import { Conversation } from "./Conversation";
import { FileCanvas } from "./FileCanvas";
import { GetGrafta } from "./GetGrafta";
import { useTalkPlayer } from "./useTalkPlayer";

/** A request's words on its button: without the "Using grafta," a first message needs. */
function chip(words: string): string {
  const rest = words.replace(/^Using grafta, /, "");
  return rest.charAt(0).toUpperCase() + rest.slice(1);
}

/**
 * The first screen: a Figma file being talked to. Three real requests on
 * order.fig, answered beside its layers and picture, playing by themselves
 * until the visitor picks one. Without scripts, every answer is in the page.
 */
export function Hero({ talk }: { talk: Talk }) {
  // False while the server renders and the page hydrates, true after.
  const mounted = useSyncExternalStore(
    () => () => undefined,
    () => true,
    () => false,
  );
  const player = useTalkPlayer(talk.requests.map((request) => request.steps.length));
  const ids = talk.requests.map((request) => request.id);
  const tabs = useTabs(ids, ids[player.active] ?? "inside", (id) => {
    player.pick(ids.indexOf(id as never));
  });

  return (
    <header
      id="hero"
      data-ready={mounted}
      className="px-4 pt-8 pb-14 sm:pt-12 lg:px-[6.25%] lg:pt-14 lg:pb-20"
    >
      <div className="grid gap-x-10 gap-y-4 lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)] lg:items-end">
        <h1 className="text-[38px] leading-[1.02] font-semibold tracking-[-0.035em] text-balance sm:text-[56px] lg:text-[60px]">
          Talk to your Figma files.
        </h1>
        <p className="max-w-[520px] text-[16px] leading-relaxed text-muted sm:text-[17px] lg:pb-1.5">
          Ask your AI what's in a file, change it, or turn it into code, in whatever language you work in. No
          Figma account, no Figma app running, no API limits.
        </p>
      </div>

      <div
        {...tabs.list("Requests to order.fig")}
        className="mt-6 flex gap-2 overflow-x-auto pb-1 sm:mt-8 [&::-webkit-scrollbar]:hidden"
      >
        {talk.requests.map((request, index) => (
          <button
            key={request.id}
            {...tabs.tab(request.id, index)}
            className="shrink-0 cursor-pointer rounded-full border border-line bg-surface px-3.5 py-1.5 text-[14px] whitespace-nowrap text-text transition-colors hover:border-text data-[state=active]:border-sap data-[state=active]:shadow-[0_0_0_1px_var(--sap)]"
          >
            “{chip(request.words)}”
          </button>
        ))}
      </div>

      <div className="mt-4 grid gap-4 lg:h-[500px] lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)]">
        <section
          aria-label="The conversation, in your AI app"
          className="order-2 flex h-[330px] flex-col overflow-hidden rounded-2xl border border-line bg-surface sm:h-[400px] lg:order-1 lg:h-auto"
        >
          <div className="flex items-center justify-between border-b border-line px-4 py-2.5 text-[13px]">
            <span className="font-medium text-text">Your AI app</span>
            <span className="text-muted">Recorded in Claude Code</span>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
            {talk.requests.map((request, index) => (
              <div
                key={request.id}
                {...tabs.panel(request.id)}
                // Until the page runs, every answer shows: app.css hides the others
                // once <html> says scripts run, and the attribute follows on mount.
                hidden={mounted ? index !== player.active : undefined}
                data-talk-panel
                data-inactive={index !== player.active}
                className="outline-none"
              >
                <Conversation
                  request={request}
                  shown={index === player.active ? player.shown : request.steps.length}
                  current={index === player.active}
                />
              </div>
            ))}
          </div>
        </section>
        <div className="order-1 h-[230px] sm:h-[400px] lg:order-2 lg:h-auto">
          <FileCanvas talk={talk} active={player.active} />
        </div>
      </div>

      <GetGrafta />
    </header>
  );
}
