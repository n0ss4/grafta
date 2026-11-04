#!/bin/sh
# Rebuild the README's showcase pictures and the site's social card in
# docs/assets/, with grafta itself.
#
#   scripts/showcase.sh path/to/any.fig
#
# The .fig lends its format only (new_file takes none of its content), so any
# file saved from Figma will do. The pictures are drawn with this machine's
# fonts; Inter must be installed.
set -eu
from="$1"
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cargo build --quiet --release -p grafta --manifest-path "$root/Cargo.toml"
python3 "$root/docs/showcase/showcase.py" "$from" "$work" > "$work/showcase.json"
"$root/target/release/grafta" tool --batch "$work/showcase.json" > "$work/results.json"
cp "$work"/showcase-*.png "$root/docs/assets/"
# The card places the showcase's pictures, so it is drawn after them.
python3 "$root/docs/showcase/card.py" "$from" "$work" "$root/docs/assets" > "$work/card.json"
"$root/target/release/grafta" tool --batch "$work/card.json" > "$work/card-results.json"
cp "$work/card.png" "$root/docs/assets/"
# The file itself, for a person to open in Figma, where ladders are kept.
if [ -d "$root/ladder" ]; then
  cp "$work/showcase.fig" "$root/ladder/showcase.fig"
  cp "$work/card-saved.fig" "$root/ladder/card.fig"
fi
ls "$root"/docs/assets/showcase-*.png "$root/docs/assets/card.png"
