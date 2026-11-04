#!/bin/sh
# Build the five grafta-<os>-<cpu> npm packages from a release's archives.
#
#   scripts/npm-packages.sh <dir with the archives> <out dir>
set -eu
dist="$1"
out="$2"
root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$root/npm/grafta/package.json")"

package() { # <target> <os> <cpu>
  dir="$out/grafta-$2-$3"
  mkdir -p "$dir/bin"
  case "$1" in
    *windows*) unzip -oq "$dist/grafta-$1.zip" -d "$dir/bin" ;;
    *) tar -xzf "$dist/grafta-$1.tar.gz" -C "$dir/bin" ;;
  esac
  sed -e "s/@PLATFORM@/$2-$3/g" -e "s/@VERSION@/$version/" -e "s/@OS@/$2/" -e "s/@CPU@/$3/" \
    "$root/npm/platform/package.json.tmpl" > "$dir/package.json"
  printf 'The grafta binary for %s-%s. Install [grafta](https://www.npmjs.com/package/grafta).\n' "$2" "$3" > "$dir/README.md"
}

package aarch64-apple-darwin darwin arm64
package x86_64-apple-darwin darwin x64
package aarch64-unknown-linux-gnu linux arm64
package x86_64-unknown-linux-gnu linux x64
package x86_64-pc-windows-msvc win32 x64
