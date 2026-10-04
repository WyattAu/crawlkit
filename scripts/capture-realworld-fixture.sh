#!/usr/bin/env bash
# Capture a real page for the real-world fixture corpus.
#
# Captures the body AND the response headers, because roughly twenty of the
# security-header analyzers read `ctx.headers` and `ctx.server`. A capture
# without them makes every one of those rules fire on every fixture, which
# buries the HTML findings the corpus exists to test and would make a
# header-analyzer regression invisible.
#
# Usage:  scripts/capture-realworld-fixture.sh <name> <url>
#
# `--compressed` is required, not optional. Some origins (python.org among the
# captured set) send `content-encoding: br` with no `Content-Length`, so a plain
# `curl` writes the *compressed* bytes: the result looks like a valid 11 KB file
# and parses as nothing. This is the same signal crawlkit's own `compression`
# module has to negotiate explicitly.
set -euo pipefail

name="$1"
url="$2"
dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/crates/crawlkit-engine/tests/fixtures/realworld"

if [ ! -d "$dir" ]; then
  echo "fixture directory not found: $dir" >&2
  exit 1
fi

ua="crawlkit-fixture-capture/6.0 (audit corpus)"

# Headers of interest. The full set is kept because a future analyzer may read
# one that is not listed here, and re-capturing is a nuisance.
headers="$(mktemp)"
trap 'rm -f "$headers"' EXIT

curl -sS -L --compressed --max-time 25 -A "$ua" \
  -D "$headers" -o "$dir/$name.html" "$url"

status="$(awk 'toupper($1) ~ /^HTTP\// { code=$2 } END { print code }' "$headers")"

python3 - "$headers" "$dir/$name.headers.json" "$status" "$url" <<'PY'
import json, sys

raw, out_path, status, url = sys.argv[1:5]

# Fold repeated headers into lists; keep the last value of the final block,
# which is what a redirect chain leaves behind.
fields, seen = {}, set()
with open(raw, encoding="utf-8", errors="replace") as fh:
    for line in fh:
        line = line.rstrip("\r\n")
        if not line:
            continue
        if line.startswith("HTTP/"):
            seen.clear()          # new response in a redirect chain
            continue
        if ":" not in line:
            continue
        name, _, value = line.partition(":")
        name, value = name.strip().lower(), value.strip()
        if not name:
            continue
        if name in seen:
            continue              # keep the first, drop the hop's duplicate
        seen.add(name)
        fields[name] = value

with open(out_path, "w", encoding="utf-8") as fh:
    json.dump(
        {
            "url": url,
            "status": int(status) if status.isdigit() else None,
            "captured_by": "scripts/capture-realworld-fixture.sh",
            "headers": fields,
        },
        fh,
        indent=2,
        sort_keys=True,
    )
    fh.write("\n")
PY

# robots.txt, because two analyzers assert on its absence. Passing `None` made
# "No robots.txt found" fire on all ten captures -- every one of which serves a
# perfectly good robots.txt.
robots_status="skipped"
robots_origin="$(printf '%s' "$url" | sed -E 's#^(https?://[^/]+).*#\1#')"
if curl -sS -L --compressed --max-time 15 -A "$ua" \
     -o "$dir/$name.robots.txt" -w '%{http_code}' "$robots_origin/robots.txt" \
     2>/dev/null | grep -q '^200$'; then
  robots_status="captured"
else
  rm -f "$dir/$name.robots.txt"
fi

size="$(wc -c < "$dir/$name.html")"
echo "captured $name: HTTP $status, $size bytes, robots.txt $robots_status"
if [ "$size" -gt 157286 ]; then
  echo "  warning: over the 150 KB fixture budget" >&2
fi