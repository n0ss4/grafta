#!/bin/sh
# Publish every crate to crates.io, dependencies first.
#
#   scripts/publish-crates.sh 0.1.0
#
# crates.io limits how fast new crates may appear, so a first release takes a
# while: a refusal for that reason waits and tries again. A version already
# there is skipped, so this can be run again after any failure.
set -u
version="$1"
wait_seconds="${GRAFTA_PUBLISH_WAIT:-660}"
tries=30

published() {
  [ "$(curl -s -o /dev/null -w '%{http_code}' -A 'grafta-release (github.com/n0ss4/grafta)' \
    "https://crates.io/api/v1/crates/$1/$version")" = 200 ]
}

for crate in grafta-kiwi grafta-schema grafta-figfile grafta-model grafta-domain grafta-app \
  grafta-render grafta-design grafta-disk grafta-figjam grafta-sites grafta-slides \
  grafta-edge grafta-memory grafta-toolkit grafta-mcp grafta; do
  if published "$crate"; then
    echo "$crate $version is already published"
    continue
  fi
  try=1
  while :; do
    log="$(mktemp)"
    cargo publish --locked -p "$crate" >"$log" 2>&1
    status=$?
    cat "$log"
    if [ "$status" -eq 0 ]; then
      rm -f "$log"
      break
    fi
    if ! grep -qiE '429|too many|rate limit' "$log" || [ "$try" -ge "$tries" ]; then
      rm -f "$log"
      echo "$crate failed" >&2
      exit 1
    fi
    rm -f "$log"
    echo "$crate: crates.io asks to wait, trying again in $wait_seconds s (try $try of $tries)"
    sleep "$wait_seconds"
    try=$((try + 1))
  done
done
