// The page's side of the reader: one promise per message, and a worker that is
// started when first needed and started again after it is stopped.
import type { Caption, FromWorker, ToWorker } from "./protocol";

/** A refusal or failure, in grafta's words; `fatal` when the reader stopped. */
export class ReaderError extends Error {
  constructor(
    message: string,
    readonly fatal: boolean,
  ) {
    super(message);
  }
}

type Waiting = { resolve: (value: never) => void; reject: (error: ReaderError) => void };

export class Reader {
  private worker: Worker | null = null;
  private waiting = new Map<number, Waiting>();
  private next = 1;
  /** Whether the module and fonts are loaded in the current worker. */
  ready = false;
  private readyListeners = new Set<() => void>();

  /** Be told each time a worker has loaded grafta; returns how to stop being told. */
  whenReady(listener: () => void): () => void {
    this.readyListeners.add(listener);
    return () => this.readyListeners.delete(listener);
  }

  open(file: File): Promise<unknown> {
    return file
      .arrayBuffer()
      .then((bytes) => this.send({ type: "open", id: 0, name: file.name, bytes }, [bytes]));
  }

  openBytes(name: string, bytes: ArrayBuffer): Promise<unknown> {
    return this.send({ type: "open", id: 0, name, bytes }, [bytes]);
  }

  call<T>(tool: string, args: object): Promise<T> {
    return this.send({ type: "call", id: 0, tool, args });
  }

  picture(args: object): Promise<{ png: Blob; caption: Caption }> {
    return this.send({ type: "picture", id: 0, args });
  }

  font(bytes: ArrayBuffer): Promise<number> {
    return this.send({ type: "font", id: 0, bytes }, [bytes]);
  }

  take(path: string): Promise<Uint8Array> {
    return this.send({ type: "take", id: 0, path });
  }

  /** Stop whatever is running: the worker goes, and the next call starts another. */
  stop(reason = "Stopped."): void {
    this.worker?.terminate();
    this.worker = null;
    this.ready = false;
    for (const { reject } of this.waiting.values()) reject(new ReaderError(reason, true));
    this.waiting.clear();
  }

  private send<T>(message: ToWorker, transfer: Transferable[] = []): Promise<T> {
    const worker = this.start();
    const id = this.next++;
    return new Promise<T>((resolve, reject) => {
      this.waiting.set(id, { resolve: resolve, reject });
      worker.postMessage({ ...message, id }, transfer);
    });
  }

  private start(): Worker {
    if (this.worker) return this.worker;
    const worker = new Worker(new URL("./reader.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = ({ data }: MessageEvent<FromWorker>) => {
      this.receive(data);
    };
    worker.onerror = (event) => {
      this.stop(`grafta could not start in this browser: ${event.message}`);
    };
    this.worker = worker;
    return worker;
  }

  private receive(data: FromWorker): void {
    if (data.type === "ready") {
      this.ready = true;
      for (const listener of this.readyListeners) listener();
      return;
    }
    const waiting = data.id === undefined ? undefined : this.waiting.get(data.id);
    if (data.type === "error" && data.fatal) {
      // The module cannot go on: everything waiting fails, and the worker goes.
      this.stop(data.message);
      return;
    }
    if (!waiting || data.id === undefined) return;
    this.waiting.delete(data.id);
    switch (data.type) {
      case "result":
        waiting.resolve(data.result as never);
        break;
      case "picture":
        waiting.resolve({ png: new Blob([data.png], { type: "image/png" }), caption: data.caption } as never);
        break;
      case "file":
        waiting.resolve(new Uint8Array(data.bytes) as never);
        break;
      case "error":
        waiting.reject(new ReaderError(data.message, false));
        break;
    }
  }
}
