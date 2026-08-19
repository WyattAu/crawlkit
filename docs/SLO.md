# Service Level Objectives (SLOs)

crawlkit's API server is a self-hostable product; these SLOs are **targets
for operators**, not a hosted SLA. They define what "healthy" means for a
production deployment and map each objective to the metrics exposed at
`GET /metrics` (API-key authenticated; see `METRICS_PUBLIC`).

## Objectives

| # | SLO | Target | Window | Source metrics |
|---|-----|--------|--------|----------------|
| 1 | API availability | ≥ 99.5% of requests succeed (non-5xx, excluding 429/503 backpressure) | 30d rolling | `crawlkit_requests_total`, `crawlkit_errors_total` |
| 2 | Crawl completion rate | ≥ 99% of accepted crawls reach `completed` (excluding target-site failures) | 30d rolling | `crawlkit_crawls_total` vs completion counter |
| 3 | Crawl submission capacity | < 1% of submissions rejected `503` at capacity | 30d rolling | 503 responses on `POST /api/v1/crawls` |
| 4 | API latency | p99 < 500ms on read endpoints (excluding long-poll crawls endpoints) | 30d rolling | `crawlkit_request_duration_seconds` |
| 5 | Per-tenant fairness | No tenant exceeds configured `requests_per_minute` by design | always | `crawlkit_crawls_started_by_tenant`, `crawlkit_pages_by_tenant` |

Notes:

- Objective 2 counts only crawls that fail for *internal* reasons (storage,
  engine errors). Target-site 4xx/5xx during crawling do not count against
  the objective; they are visible in crawl findings.
- Objective 3 is a function of `MAX_CONCURRENT_CRAWLS` and submission
  volume; sustained breaches mean the deployment is undersized.

## Alerting

Example Prometheus rules ship in [`monitoring/alerts.yml`](../monitoring/alerts.yml).
Recommended defaults:

- **Page** (act within 1h): availability burn, crawl failure spike.
- **Ticket** (act within 1 business day): capacity rejections, latency
  regression.

## Dashboards

Minimum viable dashboard panels (Prometheus/Grafana):

1. Request rate by endpoint + error ratio (`crawlkit_requests_total`)
2. Latency histogram heat map (`crawlkit_request_duration_seconds`)
3. Active crawls gauge + pages/sec (`crawlkit_active_crawls`,
   `crawlkit_pages_crawled_total`)
4. Crawl outcomes (completed/failed) and issues found
5. Per-tenant crawl volume (`crawlkit_crawls_started_by_tenant`) —
   label cardinality is bounded by admin-controlled tenant ids
