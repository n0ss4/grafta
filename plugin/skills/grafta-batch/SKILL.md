---
name: grafta-batch
description: Run grafta's tools from a terminal or a script without a model - one call with `grafta tool <name> '<json>'`, or a sequence on one open file with `grafta tool --batch calls.json`, where a later call refers to an earlier result. Use when the person wants a repeatable script, a CI job (export tokens, render screens to PNG, a PDF), to try a tool by hand, or to write a ladder of .fig files to open in Figma.
---

# grafta from a terminal: `grafta tool` and batches

`grafta tool` runs the same tools the MCP server gives a model, without one. The
`grafta` binary comes from Homebrew (`brew install n0ss4/tap/grafta`), the install
script, `cargo install grafta`, or `npx -y grafta tool …` with Node.js.

## One call

```bash
grafta tool                                     # list the tools
grafta tool read_node                           # what one takes, with an example
grafta tool open_file '{"path":"design.fig"}'   # call one
grafta tool render_node '{…}' --out frame.png   # write a picture to a file
```

Each `grafta tool` call starts afresh: a handle from one call is gone in the
next. Anything that needs a handle (almost everything) goes in a batch.

## A batch

A JSON array of calls, run in order against one toolkit, so the handle the first
call opens serves the rest:

```json
[
  { "tool": "open_file", "args": { "path": "app.fig" } },
  { "tool": "search_nodes", "args": { "handle": "$0.handle", "name": "Checkout", "type": "FRAME" } },
  { "tool": "render_node", "args": { "handle": "$0.handle", "id": "$1.results.0.id", "scale": 2 }, "out": "checkout.png" },
  { "tool": "check_contrast", "args": { "handle": "$0.handle", "id": "$1.results.0.id" } }
]
```

```bash
grafta tool --batch calls.json
```

- Each entry is `{ "tool", "args", "out"? }`. `args` defaults to `{}`.
- **References**: a string `"$N.path"` anywhere in `args` is replaced by that
  field of call N's result. The path steps through objects by key and arrays by
  index: `"$0.handle"`, `"$0.pages.0.id"`, `"$4.id"`, `"$1.results.0.id"`. The
  whole string must be the reference; it keeps the value's type (a number stays a
  number). A string not starting with `$` and a digit is left as it is.
- A reference to a call not yet run, or to a field the result lacks, stops the
  batch before that call, naming the reference.
- `"out"` writes a picture (`render_node`, `get_image`) to that file.
- The batch stops at the first call that fails.

Field names to refer to, as the tools return them: `open_file` and `new_file`
give `handle` and `pages[].id`; every `create_` tool gives `id`;
`search_nodes`, `list_screens` and `list_components` give `results[]`;
`create_variable_collection` gives `id` and `modes[].id`. When unsure, run the
batch up to that call and read the output.

## Output and exit status

- In a terminal: one short coloured line per call, then a total with the time.
- Through a pipe: `[N] tool → <result JSON>` per call on stdout, and
  `[N] tool failed: <error>` on stderr. Parse the JSON after the arrow.
- Exit status: 0 when every call succeeded; 1 for what the caller can fix (a bad
  argument, something not found, a refusal); 2 when something failed.

## Writing a batch

1. Start from the calls you would make with a model: open, find, change, look,
   save. Try each tool by hand first with `grafta tool <name>` to see what it takes.
2. Build it up a few calls at a time and run it; read each result before adding
   the next call that refers to it.
3. A batch that writes ends with `save_as` to a path that does not exist yet;
   `save_as` never overwrites, so delete the previous output before running again,
   or write to a new name.
4. Batches keep no journal, so the same batch on the same input writes the same
   file every time. That makes them good for CI and for regenerating assets.
5. For a long or data-driven batch, generate the JSON from a script (Python, Node)
   instead of writing it by hand.

## Recipes

**Tokens in CI**

```json
[
  { "tool": "open_file", "args": { "path": "design/library.fig" } },
  { "tool": "export_tokens", "args": { "handle": "$0.handle", "dir": "tokens" } }
]
```

The `tokens` folder must exist and hold none of the files being written.

**Every screen of the first page as PNG**: run `open_file` and `list_screens`
once, read the ids, then generate a batch with one `render_node` and `"out"` per
screen.

**A new file from nothing**

```json
[
  { "tool": "new_file", "args": { "path": "scratch.fig", "from": "any.fig", "page": "Screens" } },
  { "tool": "create_frame", "args": { "handle": "$0.handle", "parent": "$0.pages.0.id", "name": "Card", "width": 320, "height": 200 } },
  { "tool": "set_properties", "args": { "handle": "$0.handle", "id": "$1.id", "properties": { "layoutMode": "VERTICAL", "padding": 16, "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] } } },
  { "tool": "create_text", "args": { "handle": "$0.handle", "parent": "$1.id", "characters": "Hello" } },
  { "tool": "render_node", "args": { "handle": "$0.handle", "id": "$1.id" }, "out": "card.png" },
  { "tool": "save_as", "args": { "handle": "$0.handle", "path": "card.fig" } }
]
```

## A ladder

To show that Figma opens a new kind of write, a ladder is a batch whose rungs
each `save_as` a file into `ladder/` (not committed), from the simplest case to
the hardest. A person opens each rung in Figma; passing tests do not show that
Figma opens a file. Say which rungs have been opened and which have not.

## Limits

Batches run only on files: there is no batch on the file open in Figma with
grafta's panel. Each call is the same tool a model calls, so its refusals and its
`omitted` lists are the same: read them in the output.
