#!/usr/bin/env python3
"""Draw grafta's mark and logos as SVG, in docs/brand/.

The wordmark is set in Inter Display and converted to outlines, so it renders
the same where Inter is not installed (GitHub loads no font into an <img>).

    python3 scripts/brand.py [path/to/InterDisplay-SemiBold.otf]

Needs fontTools. The palette is docs/brand/README.md's.

docs/brand/icon-512.png (the Claude Desktop bundle's icon, 043: the mark in Sap
on transparency) and apple-touch-icon.png (the mark on Paper) are not drawn
here: there is no rasteriser among this script's dependencies. They were made
from the same paths with a headless Chrome.
"""
import os
import sys

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

INK, PAPER, SAP, BUD = "#15171C", "#F6F4EE", "#1F9D6B", "#C6F26B"
DEEP_SAP = "#13704B"  # text and buttons on light: Sap fails AA as text
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "docs", "brand")


EDGES = "M14 46 L32 18 L50 46"
STOCK = ("M22 46 A8 8 0 1 1 6 46 A8 8 0 1 1 22 46 Z", "M40 18 A8 8 0 1 1 24 18 A8 8 0 1 1 40 18 Z")
SCION = "M58 46 A8 8 0 1 1 42 46 A8 8 0 1 1 58 46 Z"


def mark(line, anchor):
    """A graph: two nodes joined, and a third joined onto them, in the accent:
    a node grafted onto a file's graph, which is what grafta reads and writes
    No background: Ink and Sap on light, Paper and Bud on dark."""
    return f"""
  <path class="line" d="{EDGES}" fill="none" stroke="{line}" stroke-width="5.5" stroke-linecap="round" stroke-linejoin="round"/>
  <path class="node" d="{STOCK[0]}" fill="{line}"/>
  <path class="node" d="{STOCK[1]}" fill="{line}"/>
  <path class="anchor" d="{SCION}" fill="{anchor}"/>"""


# The mark alone follows the reader's theme: a favicon is drawn on the browser's chrome.
ADAPTIVE = f"""
  <style>@media (prefers-color-scheme: dark) {{ .line {{ stroke: {PAPER} }} .node {{ fill: {PAPER} }} .anchor {{ fill: {BUD} }} }}</style>"""


def wordmark(font_path, text, size):
    font = TTFont(font_path)
    cmap, glyphs = font.getBestCmap(), font.getGlyphSet()
    scale = size / font["head"].unitsPerEm
    pen, x = SVGPathPen(glyphs), 0.0
    for char in text:
        name = cmap[ord(char)]
        glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, x, 0)))
        x += glyphs[name].width * scale
    return pen.getCommands(), x


def write(name, body, width, height):
    with open(os.path.join(ROOT, name), "w") as out:
        out.write(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
            f'width="{width}" height="{height}" role="img" aria-label="grafta">{body}\n</svg>\n'
        )


def main():
    font = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/Library/Fonts/InterDisplay-SemiBold.otf")
    path, advance = wordmark(font, "grafta", 60)
    width = round(64 + 16 + advance + 2)
    write("mark.svg", ADAPTIVE + mark(INK, SAP), 64, 64)
    for theme, line, anchor in (("light", INK, SAP), ("dark", PAPER, BUD)):
        body = mark(line, anchor)
        body += f'\n  <path transform="translate(80 50)" d="{path}" fill="{line}"/>'
        write(f"logo-{theme}.svg", body, width, 64)
    # One logo for any background, for where the reader's theme is unknown
    # (a README: GitHub's theme need not be the system's, and some viewers
    # ignore <picture>). Sap reads on white and on near-black.
    body = mark(SAP, SAP) + f'\n  <path transform="translate(80 50)" d="{path}" fill="{SAP}"/>'
    write("logo.svg", body, width, 64)
    swatches = [("Ink", INK), ("Paper", PAPER), ("Sap", SAP), ("Deep Sap", DEEP_SAP), ("Bud", BUD), ("Rust", "#E5603A")]
    body = "".join(
        f'\n  <rect x="{i * 88}" width="80" height="48" rx="10" fill="{hex_}" stroke="#8885" />'
        for i, (_, hex_) in enumerate(swatches)
    )
    write("palette.svg", body, 88 * len(swatches) - 8, 48)
    print(f"wrote mark.svg, logo.svg, logo-light.svg, logo-dark.svg ({width}×64) in docs/brand/")


if __name__ == "__main__":
    main()
