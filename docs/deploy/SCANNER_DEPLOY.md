# Scanner Deployment Bundle — `redis` posture (runbook §2, tracker §2)

**Purpose:** make the operator-owned launch steps (docs/SCANNER_LAUNCH_TRACKER.md
§2) copy-paste ready. Everything here is derived from code facts — env vars are
read by `crates/crawlkit-scanner/src/{api,main}.rs`, verified against the
runbook §2/§3 postures.

> **Posture decision is already made:** tracker §2 step 1 mandates the
> `redis` posture (shared politeness budget + queue fan-out, accepted with
> the §7 signature). `CRAWLKIT_BUDGET_BACKEND=redis` selects *both* the
> shared budget and queue mode — one switch, fail-closed (api.rs pins this
> with tests: unknown backend / missing URL / missing feature all abort
> startup rather than silently degrade).

## 1. Build

```bash
cargo build --release -p crawlkit-scanner --features shared-budget
# binary: target/release/crawlkit-scanner
```

## 2. Environment (the complete surface)

```bash
# --- posture (mandatory for redis posture) ---
export CRAWLKIT_BUDGET_BACKEND=redis          # selects shared budget + queue fan-out
export CRAWLKIT_REDIS_URL=redis://redis:6379/ # fail-closed: missing => startup aborts

# --- surface ---
export CRAWLKIT_SCANNER_PORT=8090             # binds 0.0.0.0:$PORT (default 8090)

# --- queue workers (defaults fine to start) ---
export CRAWLKIT_SCANNER_WORKERS=2             # per-replica scan worker pool
export CRAWLKIT_SCANNER_POLL_MS=500           # worker poll interval

# --- abuse ceilings (SCANNER_RUNBOOK §6) ---
export CRAWLKIT_SCANNER_DAILY_BUDGET=...      # service-wide daily scan ceiling
export CRAWLKIT_SCANNER_IP_LIMIT_PER_MIN=...  # per-source-IP submissions/min
```

Set the two ceiling values to the numbers signed in the runbook §7 package —
this bundle deliberately does not invent them.

## 3. Reference compose (2 replicas + Redis)

```yaml
# docker-compose.scanner.yml — reference shape, not an auto-production file.
# Persistence: Redis persistence config is a runbook §2 prerequisite for this
# posture ("documented in the deployment manifest before enabling") — the
# AOF block below is that documented config for this manifest.
services:
  redis:
    image: redis:7-alpine
    command: ["redis-server", "--appendonly", "yes", "--appendfsync", "everysec"]
    volumes: [redis-data:/data]
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
  scanner:
    image: ghcr.io/wyattau/crawlkit-scanner:VERSION   # publish or build locally
    build: { context: ../.., args: { FEATURES: shared-budget } }
    environment:
      CRAWLKIT_BUDGET_BACKEND: redis
      CRAWLKIT_REDIS_URL: redis://redis:6379/
      CRAWLKIT_SCANNER_DAILY_BUDGET: "${SCANNER_DAILY_BUDGET:?set from runbook §7}"
      CRAWLKIT_SCANNER_IP_LIMIT_PER_MIN: "${SCANNER_IP_LIMIT:?set from runbook §7}"
    depends_on: { redis: { condition: service_healthy } }
    deploy:
      replicas: 2
volumes:
  redis-data:
```

**Proxy constraint (runbook §2, accepted at §7 signature):** the scanner must
sit behind a proxy that **overwrites** `X-Forwarded-For` (never appends) and
carries the shared proxy-level rate limit. A client-controllable XFF header
defeats the per-IP limit — this is a deployment invariant, not a code check.

## 4. Launch verification (tracker §2 steps 2–4)

```bash
# 2a. liveness
curl -fsS http://scanner:8090/healthz

# 2b. one real self-scan, end-to-end (queue mode: 202 + token; poll to done)
TOKEN=$(curl -fsS -X POST -H 'content-type: application/json' \
  -d '{"url":"https://<self-host>/"}' http://scanner:8090/scan | jq -r .token)
curl -fsS http://scanner:8090/scan/$TOKEN

# 2c. metrics scraped by the dashboard (docs/SCANNER_DASHBOARDS.md)
curl -fsS http://scanner:8090/metrics

# dead-letter posture (ADR-015 §3) — same Redis
crawlkit queue dead-letter list
```

Startup log must show `budget_backend = "redis"` (the posture is logged for
exactly this runbook verification).

3. Publish GA_SIGNOFF_PACKAGE §2 copy **verbatim** (the only approved text).
4. Record the deployment in runbook §6.1 (drill log): date + revision.

## 5. Rollback

Single-replica posture (`CRAWLKIT_BUDGET_BACKEND=in-memory`, no Redis) is the
supported fallback — in-process budget, inline scans, Redis not required.
Budget state resets on restart; acceptable per runbook §2 because one replica
cannot multiply its own budget.
