# Large-Scale Testing Guide

This guide covers testing crawlkit at scale.

## Test Environments

### Small Scale

| Component | Specification |
|-----------|---------------|
| CPU | 4 cores |
| RAM | 8GB |
| Storage | 100GB SSD |
| Network | 100Mbps |
| Pages | 1,000 |

### Medium Scale

| Component | Specification |
|-----------|---------------|
| CPU | 8 cores |
| RAM | 32GB |
| Storage | 500GB SSD |
| Network | 1Gbps |
| Pages | 10,000 |

### Large Scale

| Component | Specification |
|-----------|---------------|
| CPU | 16 cores |
| RAM | 64GB |
| Storage | 1TB NVMe |
| Network | 10Gbps |
| Pages | 100,000 |

### Extreme Scale

| Component | Specification |
|-----------|---------------|
| CPU | 32 cores |
| RAM | 128GB |
| Storage | 2TB NVMe |
| Network | 25Gbps |
| Pages | 1,000,000 |

## Test Scenarios

### 1. Single Domain Crawl

```bash
# Crawl a single domain with 100k pages
crawlkit crawl https://example.com \
  --max-pages 100000 \
  --concurrency 16 \
  --delay 100 \
  --output large-crawl/
```

**Expected Results:**
- Pages/sec: >200
- Memory: <1GB
- Duration: <10 minutes

### 2. Multi-Domain Crawl

```bash
# Crawl multiple domains
for domain in $(cat domains.txt); do
  crawlkit crawl "https://$domain" \
    --max-pages 10000 \
    --concurrency 8 \
    --output "crawls/$domain/" &
done
wait
```

**Expected Results:**
- Pages/sec per domain: >100
- Total memory: <4GB
- No cross-domain interference

### 3. Concurrent Crawl

```bash
# Run 10 concurrent crawls
for i in {1..10}; do
  crawlkit crawl "https://example$i.com" \
    --max-pages 1000 \
    --concurrency 4 \
    --output "concurrent/$i/" &
done
wait
```

**Expected Results:**
- Each crawl completes successfully
- No resource exhaustion
- Stable memory usage

### 4. Long-Running Crawl

```bash
# Crawl for 24 hours
crawlkit crawl https://example.com \
  --max-pages 1000000 \
  --max-time 86400 \
  --concurrency 8 \
  --output long-crawl/
```

**Expected Results:**
- Stable memory usage
- No memory leaks
- Consistent performance

### 5. Plugin Stress Test

```bash
# Load 50 plugins
for plugin in plugins/*.wasm; do
  crawlkit plugin load "$plugin"
done

# Crawl with all plugins
crawlkit crawl https://example.com \
  --max-pages 10000 \
  --plugins all
```

**Expected Results:**
- Plugin loading: <1 second each
- Analysis overhead: <50%
- No crashes

## Performance Metrics

### Throughput

| Metric | Target | Measurement |
|--------|--------|-------------|
| Pages/sec (single) | >200 | `metrics.json` |
| Pages/sec (concurrent) | >500 | `metrics.json` |
| Analyzer throughput | >10,000/sec | `metrics.json` |
| Storage write | >50,000/sec | `metrics.json` |

### Latency

| Metric | Target | Measurement |
|--------|--------|-------------|
| HTTP fetch p95 | <500ms | `metrics.json` |
| HTML parse p95 | <15ms | `metrics.json` |
| Analysis p95 | <30ms | `metrics.json` |
| Storage write p95 | <5ms | `metrics.json` |

### Resource Usage

| Metric | Target | Measurement |
|--------|--------|-------------|
| Memory (10k pages) | <500MB | RSS monitoring |
| Memory (100k pages) | <1GB | RSS monitoring |
| CPU (8 cores) | >80% utilization | `top`/`htop` |
| Disk I/O | <100MB/s | `iostat` |

## Monitoring During Tests

### Metrics Collection

```bash
# Start metrics collection
crawlkit crawl https://example.com \
  --max-pages 10000 \
  --metrics-json metrics.json

# Monitor in real-time
watch -n 1 'curl -s http://localhost:4000/metrics | grep crawlkit_'
```

### Resource Monitoring

```bash
# Monitor memory
watch -n 1 'ps aux | grep crawlkit'

# Monitor CPU
mpstat -P ALL 1

# Monitor disk I/O
iostat -x 1
```

### Logging

```bash
# Enable verbose logging
RUST_LOG=crawlkit=debug crawlkit crawl https://example.com

# Log to file
RUST_LOG=crawlkit=info crawlkit crawl https://example.com 2>&1 | tee crawl.log
```

## Stress Testing Tools

### wrk (HTTP Load Testing)

```bash
# Test API endpoints
wrk -t12 -c400 -d30s http://localhost:4000/health
```

### Apache Bench

```bash
# Test API throughput
ab -n 10000 -c 100 http://localhost:4000/health
```

### Custom Load Generator

```python
import asyncio
import aiohttp

async def crawl_url(session, url):
    async with session.get(url) as response:
        return await response.text()

async def load_test(urls, concurrency):
    async with aiohttp.ClientSession() as session:
        tasks = [crawl_url(session, url) for url in urls]
        return await asyncio.gather(*tasks)

# Run load test
urls = [f"https://example.com/page{i}" for i in range(10000)]
results = asyncio.run(load_test(urls, 100))
```

## Regression Testing

### Baseline Establishment

```bash
# Run baseline tests
cargo bench --workspace -- --output-format bencher > baseline.txt

# Store baseline
git add baseline.txt
git commit -m "chore: update benchmark baseline"
```

### Regression Detection

```bash
# Run benchmarks
cargo bench --workspace -- --output-format bencher > current.txt

# Compare with baseline
python3 compare_benchmarks.py baseline.txt current.txt --threshold 10
```

### Automated Regression Testing

```yaml
# In CI/CD pipeline
- name: Run benchmarks
  run: cargo bench --workspace

- name: Compare with baseline
  run: |
    if ! python3 compare_benchmarks.py baseline.txt current.txt; then
      echo "Performance regression detected"
      exit 1
    fi
```

## Test Reporting

### Test Report Template

```markdown
# Performance Test Report

## Test Environment
- Date: YYYY-MM-DD
- Duration: X hours
- Scale: X pages

## Results

### Throughput
- Pages/sec: X
- Analyzer throughput: X/sec
- Storage write: X/sec

### Latency
- HTTP fetch p95: Xms
- HTML parse p95: Xms
- Analysis p95: Xms

### Resource Usage
- Peak memory: X MB
- Average CPU: X%
- Disk I/O: X MB/s

### Issues
- [List any issues encountered]

### Recommendations
- [List recommendations]
```

## Continuous Testing

### CI/CD Integration

```yaml
# Performance test job
performance:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run performance tests
      run: |
        cargo build --release
        ./target/release/crawlkit crawl https://example.com \
          --max-pages 10000 \
          --metrics-json metrics.json
    - name: Check thresholds
      run: |
        python3 check_thresholds.py metrics.json
```

### Monitoring Dashboard

```yaml
# Grafana dashboard configuration
panels:
  - title: Pages/sec
    type: graph
    targets:
      - expr: rate(crawlkit_pages_crawled_total[5m])
  - title: Memory Usage
    type: graph
    targets:
      - expr: crawlkit_memory_bytes
  - title: Error Rate
    type: graph
    targets:
      - expr: rate(crawlkit_errors_total[5m])
```
