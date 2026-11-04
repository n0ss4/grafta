#!/usr/bin/env python3
"""Write the showcase as a `grafta tool --batch` file.

The README's pictures are this batch's: a draft screen, the same nodes after a
model's edits (tokens, icons, layout), and the same frame in its Dark mode. Each
call is one a model would make; every picture is grafta's own render_node.

    python3 docs/showcase/showcase.py <any .fig, for its format> <out dir> > showcase.json

scripts/showcase.sh runs it and the batch. Calls are named here and turned into
the batch's `$N` references, so steps can move without renumbering by hand.
"""
import json
import sys

FROM, OUT = sys.argv[1], sys.argv[2]
calls, index = [], {}


def call(key, tool, out=None, /, **args):
    """Add a call; `key` lets later calls refer to its result."""
    if key:
        index[key] = len(calls)
    entry = {"tool": tool, "args": args}
    if out:
        entry["out"] = out
    calls.append(entry)


def ref(name, field="id"):
    return f"${index[name]}.{field}"


def h():
    return ref("file", "handle")


def text(key, parent, characters, size, style="Regular", color="#333333", **extra):
    fills = [{"type": "SOLID", "variable": color}] if not color.startswith("#") else [{"type": "SOLID", "color": color}]
    call(key, "create_text", handle=h(), parent=ref(parent), characters=characters,
         fontFamily="Inter", fontStyle=style, fontSize=size, fills=fills, **extra)


def props(target, **properties):
    call(None, "set_properties", handle=h(), id=ref(target), properties=properties)


def solid(variable):
    return [{"type": "SOLID", "variable": variable}]


# Lucide icons (ISC licence), as the icon set publishes their paths.
PACKAGE = [
    "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z",
    "M12 22V12",
    "m3.3 7 7.703 4.734a2 2 0 0 0 1.994 0L20.7 7",
    "m7.5 4.27 9 5.15",
]
ARROW = ["M5 12h14", "m12 5 7 7-7 7"]

# ── 1. The draft: grey boxes and default text, as a first pass looks ─────────
call("file", "new_file", path=f"{OUT}/showcase-draft.fig", **{"from": FROM}, page="Showcase")
call("screen", "create_frame", handle=h(), parent=ref("file", "pages.0.id"), name="Order", x=0, y=0, width=360, height=10)
props("screen", layoutMode="VERTICAL", padding=24, itemSpacing=16, layoutSizingVertical="HUG",
      fills=[{"type": "SOLID", "color": "#FFFFFF"}])
text("title", "screen", "My order", 22)
call("card", "create_frame", handle=h(), parent=ref("screen"), name="Card", x=0, y=0, width=312, height=10)
props("card", layoutMode="VERTICAL", padding=16, itemSpacing=8, layoutSizingVertical="HUG",
      layoutSizingHorizontal="FILL", fills=[{"type": "SOLID", "color": "#EEEEEE"}])
text("number", "card", "Order #4821", 16)
text("eta", "card", "Arriving tomorrow", 14, color="#777777")
call("track", "create_frame", handle=h(), parent=ref("card"), name="Progress", x=0, y=0, width=280, height=8)
props("track", layoutSizingHorizontal="FILL", fills=[{"type": "SOLID", "color": "#CCCCCC"}])
call("button", "create_frame", handle=h(), parent=ref("screen"), name="Button", x=0, y=0, width=312, height=10)
props("button", layoutMode="HORIZONTAL", padding=14, primaryAxisAlignItems="CENTER", counterAxisAlignItems="CENTER",
      layoutSizingHorizontal="FILL", layoutSizingVertical="HUG", fills=[{"type": "SOLID", "color": "#DDDDDD"}])
text("label", "button", "Track order", 15)
call(None, "render_node", f"{OUT}/showcase-1-draft.png", handle=h(), id=ref("screen"), scale=2)

# ── 2. Designed: tokens with a Light and a Dark mode, bound everywhere ───────
call(None, "create_variable_collection", handle=h(), name="Theme", modes=["Light", "Dark"])
for name, light, dark in [
    ("Background", "#F6F4EE", "#0D1117"),
    ("Surface", "#FFFFFF", "#161B22"),
    ("Ink", "#15171C", "#F6F4EE"),
    ("Muted", "#6B6F76", "#9DA3AB"),
    ("Border", "#E6E2D8", "#30363D"),
    ("Accent", "#13704B", "#C6F26B"),  # Deep Sap: Sap fails AA as text
    ("Accent soft", "#E3F4EA", "#1C2B23"),
    ("On accent", "#FFFFFF", "#15171C"),
    ("Track", "#ECE8DE", "#21262D"),
]:
    call(None, "create_variable", handle=h(), collection="Theme", name=name, type="COLOR",
         values={"Light": light, "Dark": dark})

props("screen", fills=solid("Background"), itemSpacing=20, padding=24)
props("title", fontStyle="Bold", fontSize=28, fills=solid("Ink"), characters="Your order")
text("meta", "screen", "Placed 2 Oct · 3 items", 14, color="Muted")
call(None, "move_node", handle=h(), id=ref("meta"), in_front_of=ref("title"))

props("card", fills=solid("Surface"), strokes=solid("Border"), strokeWeight=1, strokeAlign="INSIDE",
      cornerRadius=20, padding=20, itemSpacing=14,
      effects=[{"type": "DROP_SHADOW", "color": "#15171C", "opacity": 0.06, "offset": {"x": 0, "y": 8}, "radius": 24}])
# A header row: an icon in a soft bubble, then the two labels stacked.
call("row", "create_frame", handle=h(), parent=ref("card"), name="Header", x=0, y=0, width=10, height=10)
props("row", layoutMode="HORIZONTAL", itemSpacing=14, counterAxisAlignItems="CENTER",
      layoutSizingHorizontal="FILL", layoutSizingVertical="HUG")
call(None, "move_node", handle=h(), id=ref("row"), behind=ref("number"))
call("bubble", "create_frame", handle=h(), parent=ref("row"), name="Icon", x=0, y=0, width=44, height=44)
props("bubble", fills=solid("Accent soft"), cornerRadius=14, layoutMode="HORIZONTAL",
      layoutSizingHorizontal="FIXED", layoutSizingVertical="FIXED",
      primaryAxisAlignItems="CENTER", counterAxisAlignItems="CENTER")
call(None, "create_vector", handle=h(), parent=ref("bubble"), name="Package (Lucide)", x=0, y=0, width=22,
     viewBox=[0, 0, 24, 24], strokeWeight=2, strokeCap="ROUND", strokeJoin="ROUND",
     strokes=solid("Accent"), path=PACKAGE)
call("labels", "create_frame", handle=h(), parent=ref("row"), name="Labels", x=0, y=0, width=10, height=10)
props("labels", layoutMode="VERTICAL", itemSpacing=2, layoutSizingHorizontal="HUG", layoutSizingVertical="HUG")
call(None, "move_node", handle=h(), id=ref("number"), parent=ref("labels"))
call(None, "move_node", handle=h(), id=ref("eta"), parent=ref("labels"))
props("number", fontStyle="Bold", fontSize=17, fills=solid("Ink"))
props("eta", fills=solid("Muted"), characters="Arriving tomorrow, 9–12")

props("track", fills=solid("Track"), cornerRadius=4)
call("done", "create_frame", handle=h(), parent=ref("track"), name="Done", x=0, y=0, width=196, height=8)
props("done", fills=solid("Accent"), cornerRadius=4)
call("steps", "create_frame", handle=h(), parent=ref("card"), name="Steps", x=0, y=0, width=10, height=10)
props("steps", layoutMode="HORIZONTAL", primaryAxisAlignItems="SPACE_BETWEEN", layoutSizingHorizontal="FILL",
      layoutSizingVertical="HUG")
text(None, "steps", "Packed", 12, style="Medium", color="Muted")
text(None, "steps", "Shipped", 12, style="Medium", color="Muted")
text(None, "steps", "Out for delivery", 12, style="Bold", color="Accent")

props("button", fills=solid("Accent"), cornerRadius=14, padding=16, itemSpacing=8)
props("label", fontStyle="Bold", fills=solid("On accent"))
call(None, "create_vector", handle=h(), parent=ref("button"), name="Arrow (Lucide)", x=0, y=0, width=18,
     viewBox=[0, 0, 24, 24], strokeWeight=2.4, strokeCap="ROUND", strokeJoin="ROUND",
     strokes=solid("On accent"), path=ARROW)
text("note", "screen", "Delivered to the door · no signature", 13, color="Muted")

props("screen", explicitVariableModes={"Theme": "Light"})
call(None, "render_node", f"{OUT}/showcase-2-designed.png", handle=h(), id=ref("screen"), scale=2)

# ── 3. Dark: one property on the frame; every binding resolves in its mode ──
props("screen", explicitVariableModes={"Theme": "Dark"})
call(None, "render_node", f"{OUT}/showcase-3-dark.png", handle=h(), id=ref("screen"), scale=2)
call(None, "save_as", handle=h(), path=f"{OUT}/showcase.fig")

json.dump(calls, sys.stdout, indent=1, ensure_ascii=False)
print()
