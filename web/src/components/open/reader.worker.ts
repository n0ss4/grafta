/// <reference lib="webworker" />
// grafta's reader in a worker, so a long read or drawing never freezes the
// page. The module and Inter load on the first message; nothing else is
// fetched, and the file's bytes arrive transferred, never copied.
import type { Caption, FromWorker, ToWorker } from "./protocol";

interface Module {
  default: (input?: { module_or_path: URL }) => Promise<unknown>;
  open: (bytes: Uint8Array, name: string) => string;
  call: (tool: string, args: string) => string;
  picture: (args: string) => { png: Uint8Array; caption: string };
  addFont: (bytes: Uint8Array) => number;
  take: (path: string) => Uint8Array;
}

const FACES = ["Regular", "Medium", "SemiBold", "Bold"];
const base = new URL(`${import.meta.env.BASE_URL}reader/`, self.location.origin);

const post = (message: FromWorker, transfer: Transferable[] = []) => {
  self.postMessage(message, transfer);
};

const loaded: Promise<Module> = (async () => {
  const mod = (await import(/* @vite-ignore */ new URL("grafta_web.js", base).href)) as Module;
  await mod.default({ module_or_path: new URL("grafta_web_bg.wasm", base) });
  const fonts = await Promise.all(
    FACES.map(
      async (face) =>
        new Uint8Array(await (await fetch(new URL(`fonts/Inter-${face}.otf`, base))).arrayBuffer()),
    ),
  );
  for (const font of fonts) mod.addFont(font);
  post({ type: "ready" });
  return mod;
})();

/** A copy of a view's bytes in a buffer of their own, to transfer. */
const own = (view: Uint8Array): ArrayBuffer => view.slice().buffer;

self.onmessage = async ({ data }: MessageEvent<ToWorker>) => {
  let mod: Module;
  try {
    mod = await loaded;
  } catch (error) {
    post({
      type: "error",
      id: data.id,
      message: `grafta could not start in this browser: ${String(error)}`,
      fatal: true,
    });
    return;
  }
  try {
    switch (data.type) {
      case "open":
        post({
          type: "result",
          id: data.id,
          result: JSON.parse(mod.open(new Uint8Array(data.bytes), data.name)),
        });
        break;
      case "call":
        post({
          type: "result",
          id: data.id,
          result: JSON.parse(mod.call(data.tool, JSON.stringify(data.args))),
        });
        break;
      case "picture": {
        const picture = mod.picture(JSON.stringify(data.args));
        const png = own(picture.png);
        post({ type: "picture", id: data.id, png, caption: JSON.parse(picture.caption) as Caption }, [png]);
        break;
      }
      case "font":
        post({ type: "result", id: data.id, result: mod.addFont(new Uint8Array(data.bytes)) });
        break;
      case "take": {
        const bytes = own(mod.take(data.path));
        post({ type: "file", id: data.id, bytes }, [bytes]);
        break;
      }
    }
  } catch (error) {
    // grafta's own refusals arrive as strings; anything else is the module
    // stopping (a panic, or memory running out), after which it cannot go on.
    const fatal = typeof error !== "string";
    post({ type: "error", id: data.id, message: fatal ? stopped(error) : error, fatal });
  }
};

function stopped(error: unknown): string {
  const text = String(error);
  if (/memory/i.test(text))
    return "this file needs more memory than a browser gives a page. Install grafta to read it.";
  return `grafta stopped while reading (${text}). Try the file again, or install grafta to read it.`;
}
