#!/usr/bin/env/bash
# dogfood.sh — crawl a real production site with the freshly-built crawlkit
# and triage every finding.
#
# This is a standing release step (adopted after the kingstonpeptides.com
# a11y bug: crawlkit flagged WCAG-H67-correct decorative badges — a real
# spec-inversion bug that only real-world usage surfaced).
#
# Usage: scripts/dogfood.sh [URL]   (default: https://kingstonpeptides.com)
#
# Exit codes:
#   0 — crawl completed, report written
#   1 — build failure or crawl error
#
# Triage protocol (manual, after the crawl):
#   - Every finding is either a TRUE positive (fix the site) or a FALSE
#     positive (fix crawlkit — that's an engine bug).
#   - New analyzer behavior => run this before tagging.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_URL="${1:-https://kingstonpeptides.com}"
OUT_DIR="${DOGFOOD_OUT:-/tmp/opencode/dogfood}"

echo "== Dogfood crawl: $TARGET_URL =="
cd "$REPO_ROOT"

cargo build -p crawlkit --release 2>&1 | tail -1

BIN="$REPO_ROOT/target/release/crawlkit"
"$BIN" --version || { echo "FAIL: binary did not build"; exit 1; }

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/crawlkit.db "$OUT_DIR"/*.json
"$BIN" crawl "$TARGET_URL" \
  --max-pages 100 \
  --output "$OUT_DIR" \
  --format json

echo
echo "== Findings summary =="
python3 - "$OUT_DIR" <<'PYEOF'
import json, sys, pathlib, sqlite3
out = pathlib.Path(sys.argv[1])
# Alerts (cross-page) — quick glance
alerts = out / "alerts.json"
if alerts.exists() and (a := json.loads(alerts.read_text())):
    print(f"alerts: {len(a)}")
db = out / "crawlkit.db"
if not db.exists():
    print("No crawl DB found — check crawl output"); sys.exit(0)
conn = sqlite3.connect(db)
print("== By severity ==")
for sev, n in conn.execute("SELECT severity, COUNT(*) FROM findings GROUP BY severity ORDER BY 2 DESC"):
    print(f"  {n:5d}  {sev}")
print("== Actionable (non-info) category/code ==")
for cat, sev, code, n in conn.execute(
    "SELECT category, severity, code, COUNT(*) FROM findings WHERE severity != 'info' "
    "GROUP BY category, severity, code ORDER BY 4 DESC LIMIT 20"):
    print(f"  {n:5d}  {cat}/{sev}/{code}")
print("== A11Y (the KP dogfood canary) ==")
a11y = conn.execute("SELECT COUNT(*) FROM findings WHERE category='accessibility'").fetchone()[0]
print(f"  {a11y:5d}  accessibility findings (0 errors expected on KP — H67 fix)")
PYEOF
echo
echo "Report: $OUT_DIR"
echo "Triage: every code above is either a site fix or a crawlkit bug."
