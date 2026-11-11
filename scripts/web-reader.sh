#!/bin/sh
# Build grafta's browser reader into web/public/reader/: the module, its
# JavaScript, and the fonts it draws with.
#
#   scripts/web-reader.sh
#
# Needs the wasm32-unknown-unknown target, wasm-bindgen-cli at the version the
# workspace pins, and wasm-opt (binaryen). Fails when the module is larger than
# BUDGET_GZIP, so a dependency that bloats it is seen in the pull request.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/web/public/reader"
# Row 102's probe measured 1.6 MB gzip with these settings.
BUDGET_GZIP=1677722
# The drawing recurses per layer; natively it runs on a 64 MiB thread
# (render::STACK). A browser starts no thread, so the module's own stack is
# that size.
STACK=67108864
INTER_VERSION=4.1
INTER_SHA256=9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e
INTER_FACES="Regular Medium SemiBold Bold"
CACHE="${GRAFTA_CACHE:-$ROOT/target/web-reader-cache}"

pinned=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' "$ROOT/Cargo.toml")
installed=$(wasm-bindgen --version | cut -d' ' -f2)
if [ "$pinned" != "$installed" ]; then
  echo "wasm-bindgen $installed is installed; the workspace pins $pinned:" >&2
  echo "  cargo install wasm-bindgen-cli --version $pinned --locked" >&2
  exit 1
fi

RUSTFLAGS="-C link-arg=-zstack-size=$STACK" cargo build --quiet \
  --manifest-path "$ROOT/Cargo.toml" -p grafta-web \
  --target wasm32-unknown-unknown --profile wasm

rm -rf "$OUT"
mkdir -p "$OUT/fonts"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" --out-name grafta_web \
  "$ROOT/target/wasm32-unknown-unknown/wasm/web.wasm"
wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
  "$OUT/grafta_web_bg.wasm" -o "$OUT/grafta_web_bg.wasm"

# Inter, from its own release, checked, kept between runs.
mkdir -p "$CACHE"
zip="$CACHE/Inter-$INTER_VERSION.zip"
if [ ! -f "$zip" ]; then
  curl -fsSL -o "$zip.part" "https://github.com/rsms/inter/releases/download/v$INTER_VERSION/Inter-$INTER_VERSION.zip"
  mv "$zip.part" "$zip"
fi
echo "$INTER_SHA256  $zip" | shasum -a 256 -c - >/dev/null
for face in $INTER_FACES; do
  unzip -p "$zip" "extras/otf/Inter-$face.otf" > "$OUT/fonts/Inter-$face.otf"
done
unzip -p "$zip" LICENSE.txt > "$OUT/fonts/LICENSE.txt"

raw=$(wc -c < "$OUT/grafta_web_bg.wasm" | tr -d ' ')
gz=$(gzip -9 -c "$OUT/grafta_web_bg.wasm" | wc -c | tr -d ' ')
fonts=$(cat "$OUT"/fonts/*.otf | gzip -9 -c | wc -c | tr -d ' ')
echo "reader: $raw bytes, $gz gzip (budget $BUDGET_GZIP); fonts $fonts gzip"
if [ "$gz" -gt "$BUDGET_GZIP" ]; then
  echo "the reader is over its budget of $BUDGET_GZIP bytes gzip" >&2
  exit 1
fi
