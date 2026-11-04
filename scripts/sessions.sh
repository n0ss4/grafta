#!/bin/sh
# Write the files the website's sessions are recorded on.
#
#   scripts/sessions.sh --inputs <dir> path/to/any.fig
#
# Runs the showcase (docs/showcase/showcase.py) for its file and the setup of
# docs/showcase/sessions.py, and leaves the sessions' input files (order.fig,
# checkout.fig, pricing.json) in <dir>: the folder a session is recorded in,
# in a real client (scripts/record-code.sh). The .fig lends its
# format only. Inter must be installed, and Chrome, for the web page's
# snapshot. The sessions themselves are recorded, not scripted: the site shows
# what a model did (docs/showcase/transcripts_site.py).
set -eu
if [ "${1:-}" != "--inputs" ] || [ $# -ne 3 ]; then
  echo "usage: scripts/sessions.sh --inputs <dir> path/to/any.fig" >&2
  exit 2
fi
inputs="$(mkdir -p "$2" && cd "$2" && pwd)"
shift 2
from="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
grafta="$root/target/release/grafta"

cargo build --quiet --release -p grafta --manifest-path "$root/Cargo.toml"
python3 "$root/docs/showcase/showcase.py" "$from" "$work" > "$work/showcase.json"
"$grafta" tool --batch "$work/showcase.json" > /dev/null
"$root/scripts/snapshot.sh" "$root/docs/showcase/pricing.html" "$work/pricing.json" main > /dev/null

python3 "$root/docs/showcase/sessions.py" "$work"
cd "$work"
"$grafta" tool --batch setup.json > /dev/null
cp order.fig checkout.fig pricing.json "$inputs/"
echo "inputs written to $inputs: order.fig checkout.fig pricing.json"
