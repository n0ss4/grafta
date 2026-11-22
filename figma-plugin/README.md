# grafta's panel in Figma

A Figma plugin: open it on a file in Figma's desktop app, and the AI app you
run grafta in can use grafta's tools, by their own names, on that file. Each
change shows in place, and one call is one step of Figma's undo.

The panel talks to `grafta mcp` on your own computer and to nothing else: the
manifest allows `ws://localhost` on grafta's five ports alone. It asks you to
allow each AI app once, says in plain words what each call did (the last
fifty, scrolling), and Stop refuses every call until Resume.

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
- `src/ui.ts`, `src/ui.html`: the panel. Holds a connection to each grafta that
  answers on its ports, and what you see and decide.
- `src/tools/`: one function per tool, keyed by grafta's name, each taking the
  tool's arguments and returning its result keys. `view.ts` is read_node's
  view, rule for rule from `crates/core/app/src/view.rs`.
- `assets/icon.svg`, `assets/icon.png`: the plugin's icon, 128 by 128. Figma
  takes it in the publish dialog, not from the manifest, so a plugin imported
  for development shows none.
- `src/contract.json`: written by grafta (`GRAFTA_RECORD=1 cargo test -p
  grafta-mcp`), never by hand: the tools, what each does here, the ports.
