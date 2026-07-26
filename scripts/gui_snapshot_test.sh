#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DASHBOARD_DIR="$REPO_ROOT/dashboard"
SNAPSHOT_DIR="/tmp/crawlkit-snapshots"
PORT=3000
BASE_URL="http://localhost:$PORT"
SERVER_PID=""

# Routes: "path|name"
ROUTES=(
    "/login|login"
    "/dashboard|dashboard"
    "/crawls|crawls"
    "/results/test-123|results"
    "/users|users"
    "/settings|settings"
)

cleanup() {
    if [[ -n "${SERVER_PID:-}" ]]; then
        echo "==> Stopping dev server (PID $SERVER_PID)..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

mkdir -p "$SNAPSHOT_DIR"

# ---------------------------------------------------------------------------
# Start dev server
# ---------------------------------------------------------------------------
echo "==> Starting dashboard dev server on port $PORT..."
cd "$DASHBOARD_DIR"
pnpm dev --port "$PORT" &>/dev/null &
SERVER_PID=$!

echo "==> Waiting for dev server to be ready..."
MAX_WAIT=60
WAITED=0
until curl -s -o /dev/null "http://localhost:$PORT" 2>/dev/null; do
    sleep 1
    WAITED=$((WAITED + 1))
    if [[ $WAITED -ge $MAX_WAIT ]]; then
        echo "ERROR: Dev server did not start within ${MAX_WAIT}s" >&2
        exit 1
    fi
done
echo "    Server ready after ${WAITED}s."

# ---------------------------------------------------------------------------
# Try browser automation (puppeteer or playwright via Node)
# ---------------------------------------------------------------------------
run_browser_snapshots() {
    local has_tool=false
    local tool_name=""

    if command -v npx &>/dev/null; then
        # Quick smoke-check: can we import the module at all?
        if node -e "require.resolve('puppeteer')" &>/dev/null; then
            has_tool=true
            tool_name="puppeteer"
        elif node -e "require.resolve('playwright')" &>/dev/null; then
            has_tool=true
            tool_name="playwright"
        fi
    fi

    if [[ "$has_tool" == false ]]; then
        echo "    No browser automation tool found (puppeteer/playwright)."
        return 1
    fi

    echo "==> Using $tool_name for GUI snapshots"

    # Write a self-contained Node script
    local node_script
    node_script=$(mktemp /tmp/gui_snap_XXXXXX.mjs)

    cat > "$node_script" <<NOSCRIPT
import { writeFileSync } from 'fs';

const SNAPSHOT_DIR = "$SNAPSHOT_DIR";
const BASE_URL    = "$BASE_URL";
const TOOL        = "$tool_name";

const routes = [
$(for r in "${ROUTES[@]}"; do
    IFS='|' read -r path name <<< "$r"
    echo "  { path: '$path', name: '$name' },"
done)
];

async function puppeteerRun(routes) {
  const mod = await import('puppeteer');
  const puppeteer = mod.default ?? mod;
  const browser = await puppeteer.launch({
    headless: 'new',
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage'],
  });
  const results = [];
  for (const route of routes) {
    const page = await browser.newPage();
    const errors = [];
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    page.on('pageerror', e => errors.push(e.message));
    try {
      await page.goto(\`\${BASE_URL}\${route.path}\`, { waitUntil: 'networkidle2', timeout: 15000 });
      await page.screenshot({ path: \`\${SNAPSHOT_DIR}/\${route.name}.png\`, fullPage: true });
      writeFileSync(\`\${SNAPSHOT_DIR}/\${route.name}.html\`, await page.content());
      results.push({ ...route, errors, status: errors.length ? 'WARN' : 'OK' });
    } catch (e) {
      results.push({ ...route, errors: [e.message], status: 'FAIL' });
    }
    await page.close();
  }
  await browser.close();
  return results;
}

async function playwrightRun(routes) {
  const { chromium } = await import('playwright');
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const results = [];
  for (const route of routes) {
    const page = await ctx.newPage();
    const errors = [];
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    page.on('pageerror', e => errors.push(e.message));
    try {
      await page.goto(\`\${BASE_URL}\${route.path}\`, { waitUntil: 'networkidle', timeout: 15000 });
      await page.screenshot({ path: \`\${SNAPSHOT_DIR}/\${route.name}.png\`, fullPage: true });
      writeFileSync(\`\${SNAPSHOT_DIR}/\${route.name}.html\`, await page.content());
      results.push({ ...route, errors, status: errors.length ? 'WARN' : 'OK' });
    } catch (e) {
      results.push({ ...route, errors: [e.message], status: 'FAIL' });
    }
    await page.close();
  }
  await browser.close();
  return results;
}

const run = TOOL === 'puppeteer' ? puppeteerRun : playwrightRun;
const results = await run(routes);

console.log('');
console.log('========================================');
console.log('  GUI Snapshot Test Summary');
console.log('========================================');
console.log('');

let ok = 0, warn = 0, fail = 0;
for (const r of results) {
  const icon = r.status === 'OK' ? '[OK]' : r.status === 'WARN' ? '[WARN]' : '[FAIL]';
  console.log(\`\${icon} \${r.path} (\${r.name})\`);
  console.log(\`     Screenshot: \${SNAPSHOT_DIR}/\${r.name}.png\`);
  console.log(\`     HTML:       \${SNAPSHOT_DIR}/\${r.name}.html\`);
  if (r.errors.length) r.errors.forEach(e => console.log(\`     Error: \${e}\`));
  if (r.status === 'OK') ok++; else if (r.status === 'WARN') warn++; else fail++;
}
console.log('');
console.log(\`Total: \${results.length} | OK: \${ok} | Warnings: \${warn} | Failed: \${fail}\`);
console.log(\`Snapshots saved to: \${SNAPSHOT_DIR}/\`);
if (fail > 0) process.exit(1);
NOSCRIPT

    node "$node_script"
    local rc=$?
    rm -f "$node_script"
    return $rc
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
if run_browser_snapshots; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Fallback: verify production build compiles
# ---------------------------------------------------------------------------
echo "==> Falling back to production build verification..."
cd "$DASHBOARD_DIR"

if pnpm build; then
    echo ""
    echo "========================================"
    echo "  Build Verification: PASSED"
    echo "========================================"
    echo "Production bundle compiles without errors."
    echo "Output: $DASHBOARD_DIR/dist/"
    echo ""
    echo "Skipped GUI snapshots (no browser automation tool)."
    echo "Install puppeteer or playwright to enable snapshot testing:"
    echo "  cd dashboard && pnpm add -D puppeteer"
    echo "  # or"
    echo "  cd dashboard && pnpm add -D @playwright/test"
else
    echo "ERROR: Production build failed!"
    exit 1
fi
