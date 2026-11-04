#!/usr/bin/env python3
"""Write the website's showcase sessions as `grafta tool --batch` files.

Each session is a prompt and the calls a model would make for it, run by grafta
on the showcase's own file (docs/showcase/showcase.py), so every call, result
and picture the site shows is one grafta made. Paths are relative: the batches
run in one working directory, and the site shows the names a person would.

    python3 docs/showcase/sessions.py <out dir>

writes <out dir>/setup.json (the inputs, from the showcase's showcase.fig:
order.fig in its Light mode, and checkout.fig, the same with a text too faint
to read, for the contrast session), which scripts/sessions.sh --inputs runs.
It also writes <out dir>/<id>.json per session, the calls 067 scripted; since
the site shows sessions recorded in a real client instead
(docs/showcase/transcripts_site.py), and these batches reach it no more.
"""
import json
import pathlib
import sys

OUT = pathlib.Path(sys.argv[1])


class Batch:
    """Calls named by key, turned into the batch's `$N` references."""

    def __init__(self):
        self.calls, self.index, self.pictures = [], {}, {}

    def call(self, key, tool, picture=None, /, **args):
        if key:
            self.index[key] = len(self.calls)
        entry = {"tool": tool, "args": args}
        if picture:
            entry["out"] = picture["out"]
            self.pictures[len(self.calls)] = picture["alt"]
        self.calls.append(entry)

    def ref(self, key, field="id"):
        return f"${self.index[key]}.{field}"

    def h(self):
        return self.ref("file", "handle")

    def found(self, key):
        return self.ref(key, "results.0.id")


def picture(session, n, alt):
    return {"out": f"{session}-{n}.png", "alt": alt}


sessions = []


def session(id, title, prompt, batch):
    (OUT / f"{id}.json").write_text(json.dumps(batch.calls, indent=1, ensure_ascii=False) + "\n")
    sessions.append({"id": id, "title": title, "prompt": prompt,
                     "pictures": {str(k): v for k, v in batch.pictures.items()}})


# ── Inputs: order.fig in Light; checkout.fig with a note too faint to read ──
setup = Batch()
setup.call("file", "open_file", path="showcase.fig")
setup.call("screen", "search_nodes", handle=setup.h(), name="Order", type="FRAME")
setup.call(None, "set_properties", handle=setup.h(), id=setup.found("screen"),
           properties={"explicitVariableModes": {"Theme": "Light"}})
setup.call(None, "save_as", handle=setup.h(), path="order.fig")
setup.call("note", "search_nodes", handle=setup.h(), name="Delivered to the door", type="TEXT")
setup.call(None, "set_properties", handle=setup.h(), id=setup.found("note"),
           properties={"fills": [{"type": "SOLID", "color": "#B9B6AE"}]})
setup.call(None, "save_as", handle=setup.h(), path="checkout.fig")
(OUT / "setup.json").write_text(json.dumps(setup.calls, indent=1) + "\n")

# ── 1. A theme: a High contrast mode beside Light and Dark ──────────────────
b = Batch()
b.call("file", "open_file", path="order.fig")
b.call("screen", "search_nodes", handle=b.h(), name="Order", type="FRAME")
b.call(None, "list_variables", handle=b.h())
b.call(None, "set_variable_collection", handle=b.h(), id="Theme", add_mode="High contrast")
for name, value in [("Background", "#000000"), ("Surface", "#000000"), ("Ink", "#FFFFFF"),
                    ("Muted", "#E6E6E6"), ("Border", "#FFFFFF"), ("Accent", "#FFE14D"),
                    ("Accent soft", "#2A2A2A"), ("On accent", "#000000"), ("Track", "#3D3D3D")]:
    b.call(None, "set_variable", handle=b.h(), id=name, values={"High contrast": value})
for n, mode in enumerate(["Light", "Dark", "High contrast"], start=1):
    b.call(None, "set_properties", handle=b.h(), id=b.found("screen"),
           properties={"explicitVariableModes": {"Theme": mode}})
    b.call(None, "render_node", picture("theme", n, f"The order screen in its {mode} mode"),
           handle=b.h(), id=b.found("screen"), scale=2)
b.call(None, "save_as", handle=b.h(), path="order-themed.fig")
session("theme", "A theme", "Open order.fig with grafta. Add a High contrast mode to its Theme, render the "
        "Order screen in Light, Dark and High contrast, and save the result.", b)

# ── 2. An audit: what the file's design system is, and what escapes it ──────
b = Batch()
b.call("file", "open_file", path="order.fig")
b.call(None, "summarize", handle=b.h())
b.call(None, "list_variables", handle=b.h())
b.call(None, "list_components", handle=b.h())
b.call(None, "list_styles", handle=b.h())
b.call("texts", "search_nodes", handle=b.h(), type="TEXT")
b.call(None, "read_node", handle=b.h(), id=b.found("texts"))
b.call("screen", "search_nodes", handle=b.h(), name="Order", type="FRAME")
b.call(None, "render_node", picture("audit", 1, "The order screen the audit read"),
       handle=b.h(), id=b.found("screen"), scale=2)
session("audit", "An audit", "Open order.fig with grafta and audit its design system: its tokens, "
        "components and styles, and whether its text is bound to them.", b)

# ── 3. Contrast: find the text that fails WCAG AA, fix it with a token ──────
b = Batch()
b.call("file", "open_file", path="checkout.fig")
b.call("screen", "search_nodes", handle=b.h(), name="Order", type="FRAME")
b.call(None, "check_contrast", handle=b.h(), id=b.found("screen"))
b.call(None, "render_node", picture("contrast", 1, "Before: the delivery note is too faint to read"),
       handle=b.h(), id=b.found("screen"), scale=2)
b.call("note", "search_nodes", handle=b.h(), name="Delivered to the door", type="TEXT")
b.call(None, "set_properties", handle=b.h(), id=b.found("note"),
       properties={"fills": [{"type": "SOLID", "variable": "Muted"}]})
b.call(None, "check_contrast", handle=b.h(), id=b.found("screen"))
b.call(None, "render_node", picture("contrast", 2, "After: the note in the Muted token, which passes"),
       handle=b.h(), id=b.found("screen"), scale=2)
b.call(None, "save_as", handle=b.h(), path="checkout-fixed.fig")
session("contrast", "Contrast", "Open checkout.fig with grafta, check the Order screen's text against WCAG AA, "
        "fix what fails with the file's own colours, and show me before and after.", b)

# ── 4. From data: the card as a component, placed once per order ────────────
b = Batch()
b.call("file", "open_file", path="order.fig")
b.call("card", "search_nodes", handle=b.h(), name="Card", type="FRAME")
b.call("component", "create_component", handle=b.h(), id=b.found("card"))
b.call("list", "create_frame", handle=b.h(), parent=b.ref("file", "pages.0.id"), name="Orders",
       x=440, y=0, width=360, height=10)
b.call(None, "set_properties", handle=b.h(), id=b.ref("list"),
       properties={"layoutMode": "VERTICAL", "itemSpacing": 16, "padding": 24, "layoutSizingVertical": "HUG",
                   "fills": [{"type": "SOLID", "variable": "Background"}],
                   "explicitVariableModes": {"Theme": "Light"}})
b.call(None, "fill_template", handle=b.h(), component=b.ref("component"), parent=b.ref("list"), rows=[
    {"name": "Order 4821", "Order #4821": "Order #4821", "Arriving tomorrow": "Arriving tomorrow, 9–12"},
    {"name": "Order 4907", "Order #4821": "Order #4907", "Arriving tomorrow": "Arriving Friday, 14–17"},
    {"name": "Order 5013", "Order #4821": "Order #5013", "Arriving tomorrow": "Packed today, ships Monday"},
])
b.call(None, "render_node", picture("data", 1, "Three order cards, one per row of data"),
       handle=b.h(), id=b.ref("list"), scale=2)
b.call(None, "save_as", handle=b.h(), path="orders.fig")
session("data", "From data", "Open order.fig with grafta. Make the order card a component, and place it once "
        "per order, for orders 4821, 4907 and 5013, in a new list.", b)

# ── 5. From a web page: a browser's layout written as auto-layout ───────────
b = Batch()
b.call("file", "open_file", path="order.fig")
b.call("page", "create_page", handle=b.h(), name="Pricing")
b.call("import", "import_layout", handle=b.h(), parent=b.ref("page"), snapshot="pricing.json")
b.call(None, "render_node", picture("html", 1, "The pricing cards, imported as frames, text and auto-layout"),
       handle=b.h(), id=b.ref("import"), scale=2)
b.call(None, "save_as", handle=b.h(), path="pricing.fig")
session("html", "From a web page", "Import pricing.json, a browser's snapshot of our pricing cards, into a new "
        "page of order.fig with grafta, as layers the design team can edit.", b)

(OUT / "sessions.json").write_text(json.dumps(sessions, indent=1, ensure_ascii=False) + "\n")
