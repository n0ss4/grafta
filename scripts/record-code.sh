#!/bin/sh
# Record the website's sessions in Claude Code.
#
#   scripts/record-code.sh <work dir> <recordings dir> [id...]
#
# Runs each prompt of web/src/content/sessions.json with `claude -p` in
# <work dir>/<random>/designs, a fresh folder outside any repository holding the
# sessions' input files (taken from <work dir>, as scripts/sessions.sh
# --inputs writes them), with grafta as its only MCP server and its tools
# allowed, a new session each, and copies each session's transcript to
# <recordings dir>/<id>.code.jsonl. A folder per session: grafta's recovery
# journal is kept per file path, so a session opening the same path as an
# earlier one would replay that session's edits. The folder's
# name says nothing about the session, and <work dir> should say nothing about
# grafta either: a model reads its working directory's path.
#
# An id ending in -without (theme-without) sends that session's prompt with no
# MCP server at all and Claude Code's default permissions, nothing approved:
# the same request from someone who has not installed grafta.
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(cd "$1" && pwd)"; rec="$(mkdir -p "$2" && cd "$2" && pwd)"; shift 2
ids="${*:-theme audit contrast data html}"
printf '{ "mcpServers": { "grafta": { "command": "%s", "args": ["mcp"] } } }\n' "$root/target/release/grafta" > "$rec/mcp.json"
printf '{ "mcpServers": {} }\n' > "$rec/no-mcp.json"
for id in $ids; do
  dir="$work/$(uuidgen | tr 'A-Z' 'a-z' | cut -c1-8)/designs"
  mkdir -p "$dir"
  for f in order.fig checkout.fig pricing.json poster.png; do
    if [ -f "$work/$f" ]; then cp "$work/$f" "$dir/"; fi
  done
  cd "$dir"
  base="${id%-without}"
  prompt="$(python3 -c "import json,sys; print(next(s['prompt'] for s in json.load(open(sys.argv[1])) if s['id']==sys.argv[2]))" "$root/web/src/content/sessions.json" "$base")"
  sid="$(uuidgen | tr 'A-Z' 'a-z')"
  if [ "$base" = "$id" ]; then
    claude -p "$prompt" --session-id "$sid" --mcp-config "$rec/mcp.json" --strict-mcp-config \
      --allowedTools "mcp__grafta" > "$rec/$id.code.out" 2> "$rec/$id.code.err" || echo "$id: claude exited $?"
  else
    claude -p "$prompt" --session-id "$sid" --mcp-config "$rec/no-mcp.json" --strict-mcp-config \
      > "$rec/$id.code.out" 2> "$rec/$id.code.err" || echo "$id: claude exited $?"
  fi
  f="$(ls "$HOME"/.claude/projects/*/"$sid".jsonl 2>/dev/null | head -1)"
  if [ -n "$f" ]; then
    cp "$f" "$rec/$id.code.jsonl"
    ls *.fig | grep -v -x -e order.fig -e checkout.fig > "$rec/$id.code.files" || true
    echo "$id: recorded ($sid), wrote: $(tr '\n' ' ' < "$rec/$id.code.files")"
  else
    echo "$id: no transcript found for $sid"
  fi
done
