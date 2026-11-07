#!/usr/bin/env python3
"""Make the home page's hero pictures from the recorded conversation.

    python3 docs/showcase/talk/hero.py <session folder> <web dir>

<session folder> is the one scripts/record-code.sh made for the `talk` session
(web/src/content/talk.json): its designs/ holds order.fig and the order-light.fig
the session saved, and order.html beside designs/ is the code the model wrote.

Writes, into <web dir>/public/showcase/talk/:

- before.png: grafta's render_node of the Order frame as the file holds it;
- light.png: the same frame in the file the session saved;
- code.png: the model's order.html drawn by Chromium, at the frame's width;
- layers.json: the frame's layer tree, from read_node, the layers the light
  request changed marked.

Everything is grafta's or the session's output; nothing is drawn for the page.
Run from the repository's root, after `cargo build --release -p grafta`.
"""
import json
import pathlib
import subprocess
import sys
import tempfile

FRAME = "1735549286:1"
# The light request sets the frame's variable mode: every layer bound to the
# Theme collection is redrawn, the frame itself is what was set.
CHANGED_BY_LIGHT = {FRAME}


def tree(node):
    """A layer as the file panel shows it: name, type, id, children."""
    return {
        "id": node["id"],
        "name": node.get("name", ""),
        "type": node.get("type", ""),
        "changed": node["id"] in CHANGED_BY_LIGHT,
        "children": [tree(child) for child in node.get("children", [])],
    }


def main(session, web):
    designs = session / "designs"
    out = web / "public" / "showcase" / "talk"
    out.mkdir(parents=True, exist_ok=True)
    calls = [
        {"tool": "open_file", "args": {"path": str(designs / "order.fig")}},
        {"tool": "render_node", "args": {"handle": "$0.handle", "id": FRAME, "scale": 2}, "out": str(out / "before.png")},
        {"tool": "read_node", "args": {"handle": "$0.handle", "id": FRAME, "depth": 20}},
        {"tool": "open_file", "args": {"path": str(designs / "order-light.fig")}},
        {"tool": "render_node", "args": {"handle": "$3.handle", "id": FRAME, "scale": 2}, "out": str(out / "light.png")},
    ]
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as batch:
        json.dump(calls, batch)
    lines = subprocess.run(
        ["target/release/grafta", "tool", "--batch", batch.name], check=True, capture_output=True, text=True
    ).stdout.splitlines()
    frame = json.loads(lines[2].split("→", 1)[1])
    (out / "layers.json").write_text(json.dumps(tree(frame), indent=1) + "\n", encoding="utf-8")

    html = session / "order.html"
    shot = (
        "import { chromium } from '@playwright/test';"
        "const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 360, height: 365 }, deviceScaleFactor: 2 });"
        f"await p.goto('file://{html}'); await p.waitForLoadState('networkidle');"
        f"await p.screenshot({{ path: '{out / 'code.png'}', fullPage: false }}); await b.close();"
    )
    subprocess.run(["node", "--input-type=module", "-e", shot], check=True, cwd=web)
    print(f"wrote before.png, light.png, code.png and layers.json to {out}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2]).resolve())
