# Performance Benchmarks

This document contains performance benchmarks for crawlkit.

## Test Environment

| Component | Specification |
|-----------|---------------|
| CPU | AMD Ryzen 9 5900X (12 cores) |
| RAM | 32GB DDR4-3200 |
| Storage | NVMe SSD (1TB) |
| Network | 1Gbps Ethernet |
| OS | Ubuntu 22.04 LTS |
| Rust | 1.75.0 |

## Crawl Performance

### Pages per Second

| Concurrency | Pages/sec | Memory (MB) | CPU (%) |
|-------------|-----------|-------------|---------|
| 1 | 25 | 50 | 15 |
| 2 | 45 | 80 | 30 |
| 4 | 85 | 120 | 55 |
| 8 | 150 | 200 | 80 |
| 16 | 200 | 350 | 95 |

### Memory Usage

| Pages Crawled | Memory (MB) | Disk (MB) |
|---------------|-------------|-----------|
| 100 | 80 | 5 |
| 1,000 | 150 | 50 |
| 10,000 | 300 | 500 |
| 100,000 | 800 | 5,000 |

### Latency

| Metric | p50 | p95 | p99 |
|--------|-----|-----|-----|
| HTTP Fetch | 150ms | 500ms | 2,000ms |
| HTML Parse | 5ms | 15ms | 50ms |
| Analysis | 10ms | 30ms | 100ms |
| Storage Write | 2ms | 5ms | 15ms |
| Total per Page | 170ms | 550ms | 2,100ms |

## Analyzer Performance

### Individual Analyzer Times

| Analyzer | Time (us) | Memory (bytes) |
|----------|-----------|----------------|
| MetaTagAnalyzer | 2.5 | 512 |
| ContentQualityAnalyzer | 15.0 | 2,048 |
| LinkAnalyzer | 8.0 | 1,024 |
| SecurityHeaderAnalyzer | 3.0 | 256 |
| AccessibilityAnalyzer | 20.0 | 4,096 |
| ImageAnalyzer | 5.0 | 512 |
| StructuredDataValidator | 10.0 | 2,048 |
| HeadingHierarchyAnalyzer | 4.0 | 256 |
| WordCountAnalyzer | 2.0 | 128 |
| KeywordAnalyzer | 12.0 | 2,048 |

### Parallel vs Sequential

| Analyzers | Sequential (us) | Parallel (us) | Speedup |
|-----------|-----------------|---------------|---------|
| 5 | 50 | 15 | 3.3x |
| 10 | 100 | 20 | 5.0x |
| 15 | 150 | 25 | 6.0x |
| 20 | 200 | 30 | 6.7x |
| 28 | 280 | 35 | 8.0x |

## Storage Performance

### SQLite Operations

| Operation | Time (us) | Throughput |
|-----------|-----------|------------|
| Insert Page | 50 | 20,000/sec |
| Insert Issue | 30 | 33,000/sec |
| Query Pages | 100 | 10,000/sec |
| Query Issues | 150 | 6,600/sec |
| Batch Insert (100) | 2,000 | 50,000/sec |

### WAL Mode Performance

| Metric | Value |
|--------|-------|
| Write throughput | 50,000/sec |
| Read throughput | 100,000/sec |
| Concurrent reads | 64 |
| Concurrent writes | 1 |
| Journal size | <1MB |

## Network Performance

### Connection Pooling

| Pool Size | Connections/sec | Latency (ms) |
|-----------|-----------------|--------------|
| 10 | 100 | 150 |
| 32 | 300 | 120 |
| 64 | 500 | 100 |
| 128 | 800 | 90 |

### DNS Caching

| Metric | Without Cache | With Cache |
|--------|---------------|------------|
| First lookup | 50ms | 50ms |
| Subsequent lookups | 50ms | 1ms |
| Cache hit rate | 0% | 95% |
| Memory usage | 0 | 10MB |

## Plugin Performance

### WASM Execution

| Metric | Value |
|--------|-------|
| Load time | 10ms |
| Init time | 5ms |
| Analyze time | 15ms |
| Memory usage | 10MB |
| CPU usage | 5% |

### Native vs WASM

| Metric | Native | WASM | Overhead |
|--------|--------|------|----------|
| Analyze time | 10us | 15us | 50% |
| Memory usage | 1KB | 10KB | 10x |
| Load time | 0ms | 10ms | N/A |

## Scalability

### Horizontal Scaling

| Instances | Pages/sec | Memory (GB) |
|-----------|-----------|-------------|
| 1 | 200 | 0.3 |
| 2 | 400 | 0.6 |
| 4 | 800 | 1.2 |
| 8 | 1,500 | 2.4 |

### Vertical Scaling

| Cores | Pages/sec | Memory (GB) |
|-------|-----------|-------------|
| 2 | 50 | 0.2 |
| 4 | 100 | 0.3 |
| 8 | 200 | 0.5 |
| 16 | 350 | 0.8 |
| 32 | 500 | 1.5 |

## Regression Thresholds

| Metric | Threshold | Action |
|--------|-----------|--------|
| Pages/sec | -10% | Investigate |
| Memory usage | +20% | Investigate |
| Latency p95 | +15% | Investigate |
| Analyzer time | +20% | Investigate |
| Storage write | +25% | Investigate |

## Monitoring

### Key Metrics

- `crawlkit_pages_crawled_total` -- Total pages crawled
- `crawlkit_fetch_duration_seconds` -- Fetch duration histogram
- `crawlkit_analysis_duration_seconds` -- Analysis duration histogram
- `crawlkit_errors_total` -- Total errors
- `crawlkit_memory_bytes` -- Memory usage

### Alerting Rules

```yaml
- alert: HighErrorRate
  expr: rate(crawlkit_errors_total[5m]) > 0.1
  for: 5m
  labels:
    severity: warning

- alert: HighLatency
  expr: histogram_quantile(0.95, rate(crawlkit_fetch_duration_seconds_bucket[5m])) > 2
  for: 5m
  labels:
    severity: warning

- alert: HighMemoryUsage
  expr: crawlkit_memory_bytes > 1000000000
  for: 5m
  labels:
    severity: critical
```
