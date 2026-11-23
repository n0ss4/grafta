# grafta's panel in Figma

A Figma plugin: open it on a file in Figma's desktop app, and the AI app you
run grafta in can use grafta's tools, by their own names, on that file. Each
change shows in place, and one call is one step of Figma's undo.

The panel talks to `grafta mcp` on your own computer and to nothing else: the
manifest allows `ws://localhost` on grafta's five ports alone. It asks you to
allow each AI app once. Then:

- **Requests.** The calls an AI makes for one thing you asked are gathered
  into a request (a pause of 6 s ends one). Each request shows a picture of
  the frame it worked in before and after, and each layer it made, changed or
  deleted, every change before and after. Click a layer to go to it, or *Show
  all on canvas*. *Follow* selects each change as it is made.
- **Revert.** *Revert this request* puts every layer back as it was before the
  request and removes what it made, in one step of undo. A layer it deleted
  cannot come back that way: Figma's own undo can.
- **Ask me first.** Each change waits for *Apply* or *Skip*, in the panel or
  on the canvas's notice, up to 110 s. grafta, told the person is deciding,
  waits for that call up to 120 s (`asking` in `crates/adapters/bridge`).
- **Protected.** Layers you protect, and all inside them, are refused to the
  AI with the reason. They are kept in the file's plugin data.
- **History.** Today's requests, and the file's on this computer, with *Copy
  for the team* as text.
- **Stop** refuses every call until Resume.

## Working on it

Node 22, from this folder:

```bash
npm ci
npm run typecheck
npm run build        # dist/code.js and dist/ui.html; fails if contract.json and the panel disagree
```

In Figma's desktop app: Plugins → Development → Import plugin from manifest… →
this folder's `manifest.json`, once. After a build, close the panel and run
it again: Figma reads the folder each time. `grafta mcp` must be running in an
AI app, built from the same commit.

## How it is laid out

- `src/main.ts`: the file's side. Receives a call, runs the tool, commits one
  undo step, answers.
- `src/requests.ts`: the calls of one request, their pictures and changes,
  and the history kept on this computer. `src/restore.ts` keeps a layer as it
  was and puts it back; `src/protect.ts` keeps the protected layers;
  `src/changes.ts` says each change before and after, in read_node's view.
- `src/ui.ts`, `src/ui.html`: the panel. Holds a connection to each grafta that
  answers on its ports, and what you see and decide. `src/activity.ts` draws
  the requests, the change waiting in Ask me first, and the protected layers.
- `src/tools/`: one function per tool, keyed by grafta's name, each taking the
  tool's arguments and returning its result keys. `view.ts` is read_node's
  view, rule for rule from `crates/core/app/src/view.rs`.
- `assets/icon.svg`, `assets/icon.png`: the plugin's icon, 128 by 128. Figma
  takes it in the publish dialog, not from the manifest, so a plugin imported
  for development shows none.
- `src/contract.json`: written by grafta (`GRAFTA_RECORD=1 cargo test -p
  grafta-mcp`), never by hand: the tools, what each does here, the ports.
