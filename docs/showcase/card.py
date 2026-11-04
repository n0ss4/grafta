#!/usr/bin/env python3
"""Write the site's social card as a `grafta tool --batch` file.

The card a link to the site shows (1200 × 630, og:image) is drawn by grafta, as
the showcase is: the mark from its shapes and paths, the headline as text, and
the showcase's own pictures placed as image fills.

    python3 docs/showcase/card.py <any .fig, for its format> <out dir> <docs/assets> > card.json

scripts/showcase.sh runs it after the showcase. Inter and Inter Display must be
installed.
"""
import json
import sys

FROM, OUT, ASSETS = sys.argv[1], sys.argv[2], sys.argv[3]
calls, index = [], {}

INK, PAPER, SAP, BUD, MUTED, LINE = "#15171C", "#F6F4EE", "#1F9D6B", "#C6F26B", "#9DA3AB", "#30363D"


def call(key, tool, out=None, /, **args):
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


def solid(color, opacity=None):
    paint = {"type": "SOLID", "color": color}
    if opacity is not None:
        paint["opacity"] = opacity
    return [paint]


def props(target, **properties):
    call(None, "set_properties", handle=h(), id=ref(target), properties=properties)


def text(key, parent, characters, size, style, color, family="Inter", **extra):
    call(key, "create_text", handle=h(), parent=ref(parent), characters=characters,
         fontFamily=family, fontStyle=style, fontSize=size, fills=solid(color), **extra)


call("file", "new_file", path=f"{OUT}/card.fig", **{"from": FROM}, page="Card")
call("card", "create_frame", handle=h(), parent=ref("file", "pages.0.id"), name="Social card",
     x=0, y=0, width=1200, height=630)
props("card", fills=[
    {"type": "SOLID", "color": INK},
    {"type": "GRADIENT_RADIAL", "gradientStops": [
        {"position": 0, "color": SAP, "opacity": 0.45}, {"position": 1, "color": SAP, "opacity": 0}],
     "gradientTransform": [[1.1, 0, -0.25], [0, 1.4, -0.2]]},
], clipsContent=True)

# ── The mark, at 72 px: two nodes joined, a third grafted on ─────────────────
S = 72 / 64
call("mark", "create_frame", handle=h(), parent=ref("card"), name="Mark", x=80, y=72, width=72, height=72)
call(None, "create_vector", handle=h(), parent=ref("mark"), name="Edges", x=0, y=0, width=72, height=72,
     viewBox=[0, 0, 64, 64], path="M14 46 L32 18 L50 46", strokes=solid(PAPER), strokeWeight=5.5 * S,
     strokeCap="ROUND", strokeJoin="ROUND")
for name, cx, cy, color in [("Node", 14, 46, PAPER), ("Node", 32, 18, PAPER), ("Grafted node", 50, 46, BUD)]:
    call(None, "create_shape", handle=h(), parent=ref("mark"), type="ELLIPSE", name=name,
         x=(cx - 8) * S, y=(cy - 8) * S, width=16 * S, height=16 * S, fills=solid(color))
text(None, "card", "grafta", 46, "SemiBold", PAPER, family="Inter Display", x=170, y=78, letterSpacing=-1)

# ── The words ────────────────────────────────────────────────────────────────
text(None, "card", "MCP SERVER  ·  OPEN SOURCE", 17, "Semi Bold", BUD, x=80, y=200,
     letterSpacing={"value": 12, "unit": "PERCENT"})
call("headline", "create_text", handle=h(), parent=ref("card"), fontFamily="Inter Display", fontStyle="Bold",
     fontSize=66, fills=solid(PAPER), x=80, y=236, width=640, textAutoResize="HEIGHT",
     lineHeight={"value": 108, "unit": "PERCENT"}, letterSpacing={"value": -2, "unit": "PERCENT"},
     runs=[{"characters": "Your agent, inside\nyour "}, {"characters": "Figma files.", "fills": solid(BUD)}])
text(None, "card", "Your model reads and edits .fig files: exactly, offline, with no Figma account.",
     24, "Regular", MUTED, x=80, y=410, width=560, textAutoResize="HEIGHT",
     lineHeight={"value": 140, "unit": "PERCENT"})

# ── The four products grafta speaks, as pills ────────────────────────────────
call("pills", "create_frame", handle=h(), parent=ref("card"), name="Products", x=80, y=524, width=10, height=10)
props("pills", layoutMode="HORIZONTAL", itemSpacing=10, layoutSizingHorizontal="HUG", layoutSizingVertical="HUG")
for product, extension in [("Design", ".fig"), ("Slides", ".deck"), ("FigJam", ".jam"), ("Sites", ".site")]:
    key = f"pill-{product}"
    call(key, "create_frame", handle=h(), parent=ref("pills"), name=product, x=0, y=0, width=10, height=10)
    props(key, layoutMode="HORIZONTAL", itemSpacing=8, paddingLeft=16, paddingRight=16, paddingTop=9,
          paddingBottom=9, layoutSizingHorizontal="HUG", layoutSizingVertical="HUG", cornerRadius=999,
          strokes=solid(LINE), strokeWeight=1.5, strokeAlign="INSIDE", fills=solid(PAPER, 0.04))
    text(None, key, product, 17, "Semi Bold", PAPER)
    text(None, key, extension, 17, "Regular", MUTED)

# ── The concept: a generic web wireframe as named, editable .fig layers ─────
call("file-window", "create_frame", handle=h(), parent=ref("card"), name="landing-editable.fig",
     x=700, y=72, width=420, height=472)
props("file-window", cornerRadius=22, clipsContent=True, fills=solid("#2C2C2C"),
      strokes=solid(LINE), strokeWeight=1, strokeAlign="INSIDE",
      effects=[{"type": "DROP_SHADOW", "color": "#000000", "opacity": 0.45,
                "offset": {"x": 0, "y": 24}, "radius": 48}])
call("file-top", "create_frame", handle=h(), parent=ref("file-window"), name="File bar",
     x=0, y=0, width=420, height=50)
props("file-top", fills=solid("#202124"))
text(None, "file-top", "landing-editable.fig", 14, "Medium", PAPER, family="Inter", x=20, y=17)
text(None, "file-top", "LOCAL FILE", 10, "Semi Bold", BUD, family="Inter", x=326, y=19,
     letterSpacing={"value": 8, "unit": "PERCENT"})

call("layers", "create_frame", handle=h(), parent=ref("file-window"), name="Layers",
     x=0, y=50, width=132, height=422)
props("layers", fills=solid("#252629"))
text(None, "layers", "LAYERS", 10, "Semi Bold", MUTED, family="Inter", x=18, y=18,
     letterSpacing={"value": 10, "unit": "PERCENT"})
for i, label in enumerate(("▣  Landing", "◇  Navigation", "◇  Hero", "T  Heading", "◇  CTA",
                           "◇  Features", "◇  Footer")):
    text(None, "layers", label, 10, "Regular", PAPER if i == 0 else MUTED,
         family="Inter", x=18, y=50 + i * 30)

call("poster", "create_frame", handle=h(), parent=ref("file-window"), name="Landing",
     x=168, y=78, width=220, height=240)
props("poster", clipsContent=True, fills=solid(PAPER), strokes=solid(BUD), strokeWeight=2, strokeAlign="OUTSIDE")
for name, x, y, width, height, color in [
    ("Logo", 14, 14, 16, 10, INK), ("Nav", 128, 17, 76, 5, MUTED),
    ("Heading 1", 14, 50, 84, 14, INK), ("Heading 2", 14, 70, 66, 14, INK),
    ("Copy 1", 14, 96, 82, 5, MUTED), ("Copy 2", 14, 107, 68, 5, MUTED),
    ("CTA", 14, 126, 48, 18, SAP), ("Hero media", 116, 48, 88, 98, "#E3F4EA"),
    ("Feature 1", 14, 170, 58, 46, "#E6E2D8"), ("Feature 2", 81, 170, 58, 46, "#E6E2D8"),
    ("Feature 3", 148, 170, 58, 46, "#E6E2D8")]:
    call(None, "create_shape", handle=h(), parent=ref("poster"), type="RECTANGLE", name=name,
         x=x, y=y, width=width, height=height, fills=solid(color), cornerRadius=3)
text(None, "file-window", "1440 × 960", 10, "Medium", MUTED, family="Inter", x=244, y=334)
text(None, "file-window", "7 named layers  ·  editable text  ·  vectors", 11, "Medium", BUD,
     family="Inter", x=162, y=410)

text(None, "card", "n0ss4.github.io/grafta", 18, "Medium", MUTED, x=876, y=560)

call(None, "render_node", f"{OUT}/card.png", handle=h(), id=ref("card"), scale=1)
call(None, "save_as", handle=h(), path=f"{OUT}/card-saved.fig")

json.dump(calls, sys.stdout, indent=1, ensure_ascii=False)
print()
