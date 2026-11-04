#!/bin/sh
# Install grafta from a GitHub release.
#
#   curl -fsSL https://raw.githubusercontent.com/n0ss4/grafta/main/install.sh | sh
#
#   GRAFTA_VERSION      a tag such as v0.1.0 (default: the latest release)
#   GRAFTA_INSTALL_DIR  where grafta goes (default: $HOME/.local/bin)
#   GRAFTA_DOWNLOAD_BASE  where the archives are (default: the GitHub release),
#                       for a mirror
#
# The archive's checksum is checked against the release's SHA256SUMS before
# anything is installed. Run it again to replace the binary.
set -eu

repo="n0ss4/grafta"
dir="${GRAFTA_INSTALL_DIR:-$HOME/.local/bin}"

fail() { printf 'grafta: %s\n' "$*" >&2; exit 1; }

case "$(uname -s)/$(uname -m)" in
  Darwin/arm64)               target=aarch64-apple-darwin ;;
  Darwin/x86_64)              target=x86_64-apple-darwin ;;
  Linux/x86_64)               target=x86_64-unknown-linux-gnu ;;
  Linux/aarch64|Linux/arm64)  target=aarch64-unknown-linux-gnu ;;
  *) fail "no prebuilt binary for $(uname -s) $(uname -m); install with: cargo install grafta" ;;
esac

if [ -n "${GRAFTA_DOWNLOAD_BASE:-}" ]; then
  base="$GRAFTA_DOWNLOAD_BASE"
elif [ -n "${GRAFTA_VERSION:-}" ]; then
  base="https://github.com/$repo/releases/download/$GRAFTA_VERSION"
else
  base="https://github.com/$repo/releases/latest/download"
fi

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
else
  fail "needs curl or wget"
fi

if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  fail "needs sha256sum or shasum to check the download"
fi

archive="grafta-$target.tar.gz"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

fetch "$base/$archive" "$work/$archive" || fail "cannot download $base/$archive"
fetch "$base/SHA256SUMS" "$work/SHA256SUMS" || fail "cannot download $base/SHA256SUMS"

expected="$(grep " $archive\$" "$work/SHA256SUMS" | cut -d ' ' -f 1)"
[ -n "$expected" ] || fail "$archive is not listed in SHA256SUMS"
[ "$(digest "$work/$archive")" = "$expected" ] || fail "checksum mismatch for $archive; nothing installed"

tar -xzf "$work/$archive" -C "$work"
mkdir -p "$dir"
mv -f "$work/grafta" "$dir/grafta"
chmod +x "$dir/grafta"

printf 'installed %s to %s\n' "$("$dir/grafta" --version)" "$dir"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) printf 'note: %s is not on your PATH; add it, e.g.\n  export PATH="%s:$PATH"\n' "$dir" "$dir" ;;
esac
printf 'next: claude mcp add grafta -- %s mcp\n' "$dir/grafta"
