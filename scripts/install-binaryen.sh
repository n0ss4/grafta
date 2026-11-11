#!/bin/sh
# Install binaryen's wasm-opt at the version scripts/web-reader.sh is measured
# with, from its release, checked. A distribution's package lags: Ubuntu
# 24.04's refuses the sign-extension instructions Rust emits by default.
#
#   scripts/install-binaryen.sh <dir>     puts <dir>/bin/wasm-opt there
set -eu
VERSION=133
SHA256=2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489
dir="$1"
archive="binaryen-version_$VERSION-x86_64-linux.tar.gz"
mkdir -p "$dir"
curl -fsSL -o "$dir/$archive" "https://github.com/WebAssembly/binaryen/releases/download/version_$VERSION/$archive"
echo "$SHA256  $dir/$archive" | sha256sum -c - >/dev/null
tar -xzf "$dir/$archive" -C "$dir" --strip-components=1
rm "$dir/$archive"
"$dir/bin/wasm-opt" --version
