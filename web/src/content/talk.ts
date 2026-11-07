// The hero's conversation: three requests on order.fig, as a real client and
// grafta answered them (web/public/sessions/talk.claude-code.json, recorded by
// scripts/record-code.sh with web/src/content/talk.json). Nothing here is
// written for the page: the words, the calls and the replies are the
// recording's, the pictures grafta's and the model's (docs/showcase/talk/hero.py).
import type { Recording, Turn } from "~/components/client/types";
import spec from "./talk.json";

export type RequestId = "inside" | "light" | "code";
export type Asker = "designer" | "developer" | "receiver";

/** A run of grafta's calls, as the recording labels it. */
export interface TalkStep {
  label: string;
  tools: string[];
}

export interface TalkPicture {
  src: string;
  width: number;
  height: number;
  alt: string;
  /** Who drew it: grafta, or the browser from the model's code. */
  by: string;
}

export interface TalkRequest {
  id: RequestId;
  /** What the person typed, as sent. */
  words: string;
  asker: Asker;
  steps: TalkStep[];
  /** The reply's first paragraph, as written. */
  lead: string;
  /** The whole reply, rendered at build time. */
  replyHtml: string;
  picture: TalkPicture;
  /** Lines of the code the model wrote, for the code request. */
  code?: string | undefined;
}

export interface Layer {
  id: string;
  name: string;
  type: string;
  changed: boolean;
  children: Layer[];
}

export interface Talk {
  file: string;
  client: string;
  model: string;
  recordedOn: string;
  requests: TalkRequest[];
  layers: Layer;
}

/** The message's three requests, in its order, and who asks each. */
export const ORDER: { id: RequestId; asker: Asker }[] = [
  { id: "inside", asker: "receiver" },
  { id: "light", asker: "designer" },
  { id: "code", asker: "developer" },
];

const PICTURES: Record<RequestId, TalkPicture> = {
  inside: {
    src: "showcase/talk/before.png",
    width: 720,
    height: 730,
    alt: "The order card in order.fig, dark, as grafta draws it",
    by: "Drawn by grafta from order.fig",
  },
  light: {
    src: "showcase/talk/light.png",
    width: 720,
    height: 730,
    alt: "The same card in its Light mode, from the file the session saved",
    by: "Drawn by grafta from order-light.fig",
  },
  code: {
    src: "showcase/talk/code.png",
    width: 720,
    height: 730,
    alt: "The card as the model's HTML draws it in a browser",
    by: "The model's order.html, drawn by Chromium",
  },
};

/** How many lines of the model's code the hero shows. */
const CODE_LINES = 14;

/** The recording cut at each message the person sent, in order. */
function byRequest(turns: Turn[]): Turn[][] {
  const groups: Turn[][] = [];
  for (const turn of turns) {
    if (turn.kind === "user") groups.push([]);
    groups.at(-1)?.push(turn);
  }
  return groups;
}

/** The CSS of the page the model wrote, from its Write call's content. */
function codeOf(turns: Turn[]): string | undefined {
  for (const turn of turns) {
    if (turn.kind !== "tools" || !turn.other) continue;
    for (const call of turn.calls) {
      const content = call.args["content"];
      if (call.tool === "Write" && typeof content === "string") {
        const style = /<style>\s*([\s\S]*?)<\/style>/.exec(content)?.[1] ?? content;
        const rules = style.split("\n").filter((line) => line.trim() !== "");
        return rules.slice(0, CODE_LINES).join("\n");
      }
    }
  }
  return undefined;
}

/**
 * The hero's data from the recording: each request's words, grafta's calls,
 * the last reply, a picture. `render` turns a reply's Markdown into HTML (at
 * build time). Fails when the recording is not the one talk.json describes.
 */
export async function talkFrom(
  recording: Recording,
  layers: Layer,
  render: (markdown: string) => string | Promise<string>,
): Promise<Talk> {
  const groups = byRequest(recording.turns);
  const words = [spec.prompt, ...spec.followUps];
  if (groups.length !== ORDER.length) {
    throw new Error(`talk: ${groups.length} requests recorded, talk.json has ${ORDER.length}`);
  }
  const requests = await Promise.all(
    ORDER.map(async ({ id, asker }, index) => {
      const turns = groups[index] ?? [];
      const first = turns[0];
      if (first?.kind !== "user" || first.text.trim() !== words[index]) {
        throw new Error(`talk: request ${index + 1} is not "${words[index]}"`);
      }
      const replies = turns.flatMap((turn) => (turn.kind === "reply" ? [turn.text] : []));
      const reply = replies.at(-1) ?? "";
      // grafta's calls under their labels; a label the client's own tools split in
      // two is one step.
      const steps: TalkStep[] = [];
      for (const turn of turns) {
        if (turn.kind !== "tools" || turn.other) continue;
        const tools = turn.calls.map((call) => call.tool);
        const last = steps.at(-1);
        if (last?.label === turn.label) last.tools.push(...tools);
        else steps.push({ label: turn.label, tools });
      }
      return {
        id,
        words: first.text.trim(),
        asker,
        steps,
        lead: reply.split(/\n\s*\n/)[0]?.trim() ?? "",
        replyHtml: await render(reply),
        picture: PICTURES[id],
        code: id === "code" ? codeOf(turns) : undefined,
      } satisfies TalkRequest;
    }),
  );
  return {
    file: "order.fig",
    client: recording.client,
    model: recording.model,
    recordedOn: recording.recordedOn,
    requests,
    layers,
  };
}
