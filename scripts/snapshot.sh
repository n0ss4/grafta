#!/bin/sh
# Take a layout snapshot of a local HTML page with headless Chrome,
# for import_layout.
#
#   scripts/snapshot.sh page.html snapshot.json ['css selector'] [viewport width]
#
# The page is loaded in an iframe of a page in a temporary directory, so nothing
# is written beside it and its relative paths still work. Chrome is the
# person's own: set GRAFTA_CHROME to its path if it is not found.
set -eu
page="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
out="$2"
selector="${3:-body > *}"
width="${4:-1280}"
here="$(cd "$(dirname "$0")" && pwd)"

chrome="${GRAFTA_CHROME:-}"
if [ -z "$chrome" ]; then
  for candidate in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" google-chrome google-chrome-stable chromium chromium-browser; do
    if command -v "$candidate" >/dev/null 2>&1 || [ -x "$candidate" ]; then chrome="$candidate"; break; fi
  done
fi
[ -n "$chrome" ] || { echo "grafta: Chrome not found; set GRAFTA_CHROME" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
python3 - "$page" "$selector" "$width" "$here/snapshot.js" "$work/run.html" <<'PY'
import json, sys, pathlib
page, selector, width, script, out = sys.argv[1:]
pathlib.Path(out).write_text(f"""<!doctype html><html><body style="margin:0">
<iframe id="page" src="{pathlib.Path(page).as_uri()}" style="border:0;width:{int(width)}px;height:4000px"></iframe>
<script>{pathlib.Path(script).read_text()}</script>
<script>
document.getElementById('page').addEventListener('load', () => {{
  let text;
  try {{ text = JSON.stringify(graftaSnapshot({json.dumps(selector)}, document.getElementById('page').contentWindow)); }}
  catch (e) {{ text = JSON.stringify({{ error: String(e) }}); }}
  document.body.innerHTML = '<pre id="grafta"></pre>';
  document.getElementById('grafta').textContent = text;
}});
</script></body></html>""")
PY
"$chrome" --headless --disable-gpu --allow-file-access-from-files --virtual-time-budget=5000 --dump-dom "file://$work/run.html" 2>/dev/null > "$work/dom.html"
python3 - "$work/dom.html" "$out" <<'PY'
import html, json, re, sys
dom = open(sys.argv[1]).read()
match = re.search(r'<pre id="grafta">(.*?)</pre>', dom, re.S)
if not match:
    sys.exit("grafta: the page did not finish loading in Chrome")
snapshot = json.loads(html.unescape(match.group(1)))
if "error" in snapshot:
    sys.exit("grafta: " + snapshot["error"])
json.dump(snapshot, open(sys.argv[2], "w"))
print(f"{sys.argv[2]}: {snapshot['nodes']} nodes from {snapshot['url']}")
PY
