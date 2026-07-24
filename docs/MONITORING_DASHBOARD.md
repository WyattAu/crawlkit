# Monitoring Dashboard

## Overview

This document defines the monitoring dashboard for crawlkit production deployment.

## Metrics Collection

### Application Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `crawlkit_pages_crawled_total` | Counter | Total pages crawled |
| `crawlkit_pages_failed_total` | Counter | Total pages failed |
| `crawlkit_fetch_duration_seconds` | Histogram | HTTP fetch duration |
| `crawlkit_analysis_duration_seconds` | Histogram | Analysis duration |
| `crawlkit_errors_total` | Counter | Total errors |
| `crawlkit_memory_bytes` | Gauge | Memory usage |
| `crawlkit_active_crawls` | Gauge | Active crawl count |

### System Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `process_cpu_seconds_total` | Counter | CPU usage |
| `process_resident_memory_bytes` | Gauge | RSS memory |
| `process_open_fds` | Gauge | Open file descriptors |
| `process_threads` | Gauge | Thread count |

### Business Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `crawlkit_users_total` | Gauge | Total users |
| `crawlkit_tenants_total` | Gauge | Total tenants |
| `crawlkit_plugins_total` | Gauge | Total plugins |
| `crawlkit_api_requests_total` | Counter | Total API requests |

## Alerting Rules

### Critical Alerts

```yaml
- alert: HighErrorRate
  expr: rate(crawlkit_errors_total[5m]) > 0.1
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "High error rate detected"
    description: "Error rate is {{ $value }} errors/sec"

- alert: ServiceDown
  expr: up{job="crawlkit"} == 0
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "Service is down"
    description: "crawlkit service is not responding"
```

### Warning Alerts

```yaml
- alert: HighLatency
  expr: histogram_quantile(0.95, rate(crawlkit_fetch_duration_seconds_bucket[5m])) > 2
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High latency detected"
    description: "p95 latency is {{ $value }} seconds"

- alert: HighMemoryUsage
  expr: crawlkit_memory_bytes > 1000000000
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High memory usage"
    description: "Memory usage is {{ $value }} bytes"

- alert: HighCPUUsage
  expr: rate(process_cpu_seconds_total[5m]) > 0.8
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High CPU usage"
    description: "CPU usage is {{ $value }}%"
```

### Info Alerts

```yaml
- alert: HighThroughput
  expr: rate(crawlkit_pages_crawled_total[5m]) > 500
  for: 5m
  labels:
    severity: info
  annotations:
    summary: "High throughput detected"
    description: "Throughput is {{ $value }} pages/sec"
```

## Dashboard Panels

### Overview Dashboard

| Panel | Type | Query |
|-------|------|-------|
| Pages/sec | Graph | `rate(crawlkit_pages_crawled_total[5m])` |
| Error Rate | Graph | `rate(crawlkit_errors_total[5m])` |
| Memory Usage | Gauge | `crawlkit_memory_bytes` |
| Active Crawls | Stat | `crawlkit_active_crawls` |
| API Requests | Graph | `rate(crawlkit_api_requests_total[5m])` |

### Crawl Dashboard

| Panel | Type | Query |
|-------|------|-------|
| Pages Crawled | Graph | `rate(crawlkit_pages_crawled_total[5m])` |
| Fetch Latency | Heatmap | `rate(crawlkit_fetch_duration_seconds_bucket[5m])` |
| Analysis Latency | Heatmap | `rate(crawlkit_analysis_duration_seconds_bucket[5m])` |
| Success Rate | Gauge | `1 - (rate(crawlkit_errors_total[5m]) / rate(crawlkit_pages_crawled_total[5m]))` |

### System Dashboard

| Panel | Type | Query |
|-------|------|-------|
| CPU Usage | Graph | `rate(process_cpu_seconds_total[5m])` |
| Memory Usage | Graph | `process_resident_memory_bytes` |
| File Descriptors | Gauge | `process_open_fds` |
| Thread Count | Gauge | `process_threads` |

## Log Aggregation

### Log Levels

- **ERROR**: System errors, failures
- **WARN**: Unexpected conditions
- **INFO**: Normal operations
- **DEBUG**: Detailed debugging

### Log Structure

```json
{
  "timestamp": "2026-07-24T12:00:00Z",
  "level": "info",
  "message": "Crawl completed",
  "crawl_id": "abc-123",
  "pages_crawled": 50,
  "duration_ms": 15000,
  "host": "production-1"
}
```

### Log Retention

| Level | Retention | Storage |
|-------|-----------|---------|
| ERROR | 90 days | Hot storage |
| WARN | 30 days | Hot storage |
| INFO | 7 days | Warm storage |
| DEBUG | 1 day | Cold storage |

## Uptime Monitoring

### Health Checks

| Endpoint | Interval | Timeout | Expected |
|----------|----------|---------|----------|
| `/health` | 30s | 5s | 200 OK |
| `/metrics` | 60s | 5s | 200 OK |
| `/api/v1/auth/login` | 60s | 5s | 200/401 |

### Uptime Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Availability | 99.9% | Monthly |
| Response Time | <200ms p95 | Monthly |
| Error Rate | <0.1% | Monthly |

## Incident Response

### Severity Levels

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Service down | 15 minutes |
| P2 | Major feature broken | 1 hour |
| P3 | Minor feature broken | 4 hours |
| P4 | Enhancement request | 1 day |

### Escalation Path

1. **On-call engineer** -- First response
2. **Team lead** -- Escalation after 15 minutes
3. **Engineering manager** -- Escalation after 30 minutes
4. **CTO** -- Escalation after 1 hour

### Communication Templates

**Incident Start:**
```
[INCIDENT] Service degradation detected
- Impact: [description]
- Start time: [time]
- Status: Investigating
```

**Incident Update:**
```
[INCIDENT UPDATE] Progress report
- Status: [investigating/identified/monitoring]
- Root cause: [description]
- Next update: [time]
```

**Incident Resolved:**
```
[INCIDENT RESOLVED] Service restored
- Duration: [time]
- Root cause: [description]
- Action items: [list]
```

## Rollback Procedures

### Rollback Triggers

| Trigger | Condition | Action |
|---------|-----------|--------|
| Error rate spike | >5% for 5 min | Rollback to previous version |
| Memory leak | >2GB for 10 min | Restart and investigate |
| Data corruption | Any detected | Immediate rollback |
| Security breach | Any detected | Immediate rollback and audit |

### Rollback Steps

1. **Identify** -- Determine the issue and affected version
2. **Communicate** -- Notify stakeholders of rollback
3. **Execute** -- Deploy previous stable version
4. **Verify** -- Confirm service restoration
5. **Investigate** -- Root cause analysis
6. **Document** -- Post-incident report

### Rollback Commands

```bash
# Rollback to previous version
kubectl rollout undo deployment/crawlkit-api

# Rollback to specific version
kubectl rollout undo deployment/crawlkit-api --to-revision=<revision>

# Verify rollback
kubectl rollout status deployment/crawlkit-api
```

## Monitoring Tools

### Required Tools

| Tool | Purpose | Configuration |
|------|---------|---------------|
| Prometheus | Metrics collection | Scrape interval: 15s |
| Grafana | Dashboard visualization | Retention: 30 days |
| AlertManager | Alert routing | Slack integration |
| Loki | Log aggregation | Retention: 30 days |
| Tempo | Distributed tracing | Retention: 7 days |

### Dashboard Maintenance

| Task | Frequency | Owner |
|------|-----------|-------|
| Review alert rules | Weekly | SRE |
| Update dashboards | Monthly | SRE |
| Review log retention | Monthly | SRE |
| Capacity planning | Quarterly | Engineering |
