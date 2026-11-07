#!/usr/bin/env python3
"""Write the site's social card as a `grafta tool --batch` file.

The card a link to the site shows (1200 × 630, og:image) is drawn by grafta, as
the showcase is: the mark from its shapes and paths, the headline as text, and
the idea of docs/showcase/talk, one file and three asks, in shapes.

    python3 docs/showcase/card.py <any .fig, for its format> <out dir> <docs/assets> > card.json

scripts/showcase.sh runs it after the showcase. Inter and Inter Display must be
installed, and PT Mono for the code.
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

# ── The words, from the message ──────────────────────────────────────────────
text(None, "card", "OPEN SOURCE  ·  FOR YOUR AI APP", 17, "Semi Bold", BUD, x=80, y=200,
     letterSpacing={"value": 12, "unit": "PERCENT"})
call("headline", "create_text", handle=h(), parent=ref("card"), fontFamily="Inter Display", fontStyle="Bold",
     fontSize=72, fills=solid(PAPER), x=80, y=236, width=600, textAutoResize="HEIGHT",
     lineHeight={"value": 106, "unit": "PERCENT"}, letterSpacing={"value": -2, "unit": "PERCENT"},
     runs=[{"characters": "Talk to your\n"}, {"characters": "Figma files.", "fills": solid(BUD)}])
text(None, "card", "Ask your AI what's in one, change it, or turn it into code. No Figma account, no app running.",
     24, "Regular", MUTED, x=80, y=414, width=540, textAutoResize="HEIGHT",
     lineHeight={"value": 140, "unit": "PERCENT"})

# ── The idea: one file, three asks, as in docs/showcase/talk ─────────────────
call("file-window", "create_frame", handle=h(), parent=ref("card"), name="order.fig",
     x=700, y=72, width=420, height=472)
props("file-window", cornerRadius=22, clipsContent=True, fills=solid("#202227"),
      strokes=solid(LINE), strokeWeight=1, strokeAlign="INSIDE",
      effects=[{"type": "DROP_SHADOW", "color": "#000000", "opacity": 0.45,
                "offset": {"x": 0, "y": 24}, "radius": 48}])
call("file-top", "create_frame", handle=h(), parent=ref("file-window"), name="File bar",
     x=0, y=0, width=420, height=50)
props("file-top", fills=solid("#1A1C20"))
text(None, "file-top", "order.fig", 14, "Medium", PAPER, x=20, y=17)
text(None, "file-top", "A FIGMA FILE", 10, "Semi Bold", BUD, x=318, y=19,
     letterSpacing={"value": 8, "unit": "PERCENT"})

# The file: an order card, drawn small
call("order", "create_frame", handle=h(), parent=ref("file-window"), name="Your order",
     x=24, y=74, width=200, height=150)
props("order", cornerRadius=14, clipsContent=True, fills=solid(PAPER))
text(None, "order", "Your order", 17, "Bold", INK, family="Inter Display", x=16, y=14)
text(None, "order", "Placed 2 Oct · 3 items", 9, "Regular", "#6B7178", x=16, y=38)
call("status", "create_frame", handle=h(), parent=ref("order"), name="Status", x=16, y=58, width=168, height=42)
props("status", cornerRadius=9, fills=solid("#FFFFFF"), strokes=solid("#E6E2D8"), strokeWeight=1,
      strokeAlign="INSIDE")
call(None, "create_shape", handle=h(), parent=ref("status"), type="RECTANGLE", name="Icon",
     x=8, y=8, width=16, height=16, fills=solid("#E3F4EA"), cornerRadius=5)
text(None, "status", "Order #4821", 9, "Semi Bold", INK, x=30, y=8)
call(None, "create_shape", handle=h(), parent=ref("status"), type="RECTANGLE", name="Track",
     x=8, y=31, width=152, height=4, fills=solid("#ECE9E1"), cornerRadius=2)
call(None, "create_shape", handle=h(), parent=ref("status"), type="RECTANGLE", name="Done",
     x=8, y=31, width=106, height=4, fills=solid(SAP), cornerRadius=2)
call(None, "create_shape", handle=h(), parent=ref("order"), type="RECTANGLE", name="Track order",
     x=16, y=110, width=168, height=26, fills=solid("#17744F"), cornerRadius=7)
text(None, "order", "Track order", 9, "Semi Bold", "#FFFFFF", x=74, y=117)

# The code it became
call("code", "create_frame", handle=h(), parent=ref("file-window"), name="Code",
     x=236, y=74, width=160, height=150)
props("code", cornerRadius=14, fills=solid("#0E0F12"), strokes=solid(LINE), strokeWeight=1,
      strokeAlign="INSIDE")
for i, (line, color) in enumerate([(".card {", BUD), ("  padding: 20px;", PAPER), ("  gap: 14px;", PAPER),
                                   ("  radius: 20px;", PAPER), ("}", BUD), (".done {", BUD),
                                   ("  width: 196px;", PAPER), ("}", BUD)]):
    text(None, "code", line, 9, "Regular", color, family="PT Mono", x=14, y=14 + i * 15)

# Three asks, as a person types them, on the right as a chat puts them
call("asks", "create_frame", handle=h(), parent=ref("file-window"), name="Asks", x=24, y=248, width=372, height=10)
props("asks", layoutMode="VERTICAL", itemSpacing=14, counterAxisAlignItems="MAX",
      layoutSizingVertical="HUG", fills=[])
for i, ask in enumerate(("What's in this file?", "Make it light, and check the contrast.",
                         "Now turn it into code.")):
    key = f"ask-{i}"
    call(key, "create_frame", handle=h(), parent=ref("asks"), name=f"Ask {i + 1}", x=0, y=0, width=10, height=10)
    props(key, layoutMode="HORIZONTAL", paddingLeft=16, paddingRight=16, paddingTop=11, paddingBottom=11,
          layoutSizingHorizontal="HUG", layoutSizingVertical="HUG", cornerRadius=14,
          fills=solid(PAPER, 0.08), strokes=solid(LINE), strokeWeight=1, strokeAlign="INSIDE")
    text(None, key, ask, 15, "Medium", PAPER)
text(None, "file-window", "Your file stays yours: changes are saved as a new one", 11, "Medium", BUD, x=24, y=436)

text(None, "card", "n0ss4.github.io/grafta", 18, "Medium", MUTED, x=80, y=548)

call(None, "render_node", f"{OUT}/card.png", handle=h(), id=ref("card"), scale=1)
call(None, "save_as", handle=h(), path=f"{OUT}/card-saved.fig")

json.dump(calls, sys.stdout, indent=1, ensure_ascii=False)
print()
