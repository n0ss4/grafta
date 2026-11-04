#!/bin/sh
# Print the Homebrew formula for a release, from its SHA256SUMS.
#
#   scripts/homebrew-formula.sh 0.1.0 SHA256SUMS > Formula/grafta.rb
set -eu
version="$1"
sums="$2"
sha() { grep " grafta-$1.tar.gz\$" "$sums" | cut -d ' ' -f 1; }
sed -e "s/@VERSION@/$version/g" \
    -e "s/@SHA_MACOS_ARM@/$(sha aarch64-apple-darwin)/" \
    -e "s/@SHA_MACOS_INTEL@/$(sha x86_64-apple-darwin)/" \
    -e "s/@SHA_LINUX_ARM@/$(sha aarch64-unknown-linux-gnu)/" \
    -e "s/@SHA_LINUX_INTEL@/$(sha x86_64-unknown-linux-gnu)/" \
    "$(dirname "$0")/../packaging/homebrew/grafta.rb"
