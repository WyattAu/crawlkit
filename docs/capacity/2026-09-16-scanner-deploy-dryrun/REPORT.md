# Scanner Deploy Dry-Run — 2026-09-16

**Purpose:** de-risk the tracker §2 launch procedure by executing every
automatable step locally before the operator deploy. Artifacts:
`Dockerfile.scanner`, `docker-compose.scanner.yml` (committed), with the
reference shape documented in `docs/deploy/SCANNER_DEPLOY.md`.

## What was exercised

| Step | Result |
|---|---|
| `docker build -f Dockerfile.scanner --build-arg FEATURES=shared-budget` | PASS (image `crawlkit-scanner:shared-budget`) |
| `docker compose -f docker-compose.scanner.yml up -d --scale scanner=2` | PASS — Redis (healthy) + 2 scanner replicas |
| Startup posture log | `budget_backend=Redis queue_mode=true` on both replicas |
| `GET /healthz` × 2 | `ok` on both replicas |
| `POST /scan` (replica 1) | 202-shape: `{"state":"queued","token":…}` |
| `GET /scan/{token}` (replica 2) | `{"state":"complete","report":{…real findings…}}` — **submission on one replica, execution via the Redis queue, result served by the other**: the multi-replica contract proven end-to-end |
| `GET /metrics` | outcome + submission counters present (`complete: 1`, queue-mode latency buckets) |
| Shared abuse state | `crawlkit:scanner:daily:{day}` counter in Redis (cross-replica budget); per-scan namespaces under `crawlkit:scanner:{token}` |
| Teardown | `compose down -v` clean |

## Fixes the dry-run forced (both committed)

1. **`Dockerfile.scanner` missing `workspace-hack/`** — the workspace
   member list includes it; a `crates/`-only COPY fails manifest load.
2. **Builder image `rust:1.85` → `rust:1.94`** — CI's MSRV is 1.94 and
   current dependencies (cranelift, addr2line) require it; 1.85 fails
   resolution.

## Honest scope

- Loopback-only: no XFF-overwriting proxy in the dry-run (the per-IP
  limiter is in-process by design; the proxy invariant remains an
  operator deploy requirement, not testable here).
- No load test at the daily budget (§4 test plan item) — the budget
  ceiling itself (5000/day default, lowering-only) is unit-tested in the
  scanner crate; sustained-load evidence stays an operator concern.
- `example.com` scan exercised the fetch + analyze path against the
  public internet; findings shown are genuine engine output.
