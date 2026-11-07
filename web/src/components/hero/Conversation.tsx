import { Rich } from "../Rich";
import type { TalkRequest } from "~/content/talk";

/**
 * One request in the AI app: the person's words, grafta's calls as they
 * arrive, and the reply. `shown` steps are visible; the reply follows the
 * last. Hidden steps keep their space, so nothing below moves as they arrive.
 */
export function Conversation({
  request,
  shown,
  current,
}: {
  request: TalkRequest;
  shown: number;
  current: boolean;
}) {
  const answered = shown >= request.steps.length;
  return (
    <div className="flex flex-col gap-3">
      <p className="ml-auto max-w-[85%] rounded-2xl rounded-br-md bg-button px-3.5 py-2 text-[15px] leading-snug text-on-button">
        {request.words}
      </p>
      <ol className="flex flex-col gap-1.5" aria-label="grafta's calls">
        {request.steps.map((step, index) => {
          const visible = index < shown;
          return (
            <li
              key={step.label}
              aria-hidden={!visible}
              className="talk-fade flex items-start gap-2 text-[13px] leading-snug"
              data-visible={visible}
            >
              <span aria-hidden="true" className="mt-[3px] size-2.5 shrink-0 rounded-full bg-sap" />
              <span>
                <span className="text-text">{step.label}</span>{" "}
                <span className="text-muted">{step.tools.join(", ")}</span>
              </span>
            </li>
          );
        })}
      </ol>
      <div aria-live={current ? "polite" : undefined} className="talk-fade" data-visible={answered}>
        {answered && (
          <>
            <p className="text-[15px] leading-relaxed text-text">
              <Rich text={request.lead} />
            </p>
            {request.code && (
              <pre className="mt-3 overflow-x-auto rounded-lg bg-code p-3 font-mono text-[12px] leading-[1.55] text-text">
                <code>{request.code}</code>
              </pre>
            )}
            <details className="mt-3 text-[14px]">
              <summary className="cursor-pointer font-medium text-accent-text">Read the whole reply</summary>
              <div
                className="prose-grafta talk-reply mt-2"
                // The model's reply from the recording, rendered at build time.
                dangerouslySetInnerHTML={{ __html: request.replyHtml }}
              />
            </details>
          </>
        )}
      </div>
    </div>
  );
}
