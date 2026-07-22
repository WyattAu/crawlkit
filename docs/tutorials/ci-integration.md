# CI/CD Integration Guide

Automate site crawls in your CI pipeline to catch SEO regressions, broken links, and performance degradations before they reach production.

## Overview

crawlkit is a single static binary with no runtime dependencies. Drop it into any CI environment and run it as a step.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Push / PR  │────▶│   Crawl     │────▶│   Report    │
│             │     │  (crawlkit) │     │  (artifact) │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Fail CI if │
                    │  critical   │
                    │  issues     │
                    └─────────────┘
```

## GitHub Actions

```yaml
# .github/workflows/seo-audit.yml
name: SEO Audit

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  crawl-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install crawlkit
        run: |
          cargo install crawlkit --git https://github.com/WyattAu/crawlkit

      - name: Crawl site
        run: |
          crawlkit crawl ${{ vars.SITE_URL }} \
            --max-pages 100 \
            --delay 500 \
            --concurrency 4 \
            --output crawl-results/

      - name: Check for critical issues
        run: |
          # Parse JSON output and fail if critical issues exist
          CRITICAL=$(jq '.total_issues' crawl-results/crawl-results.json)
          if [ "$CRITICAL" -gt 0 ]; then
            echo "Found $CRITICAL critical issues"
            cat crawl-results/crawl-results.json
            exit 1
          fi

      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: seo-report
          path: crawl-results/
          retention-days: 30
```

## GitLab CI

```yaml
# .gitlab-ci.yml
seo-audit:
  stage: test
  image: rust:latest
  script:
    - cargo install crawlkit --git https://github.com/WyattAu/crawlkit
    - crawlkit crawl "$SITE_URL" --max-pages 100 --output crawl-results/
    - |
      CRITICAL=$(jq '.total_issues' crawl-results/crawl-results.json)
      if [ "$CRITICAL" -gt 0 ]; then
        echo "SEO audit failed with $CRITICAL issues"
        exit 1
      fi
  artifacts:
    when: always
    paths:
      - crawl-results/
    expire_in: 30 days
  variables:
    SITE_URL: "https://staging.example.com"
```

## Using pre-built binaries (faster CI)

Building from source takes ~5 minutes. Use pre-built binaries instead:

```yaml
# .github/workflows/seo-audit.yml
- name: Download crawlkit
  run: |
    curl -sSL https://github.com/WyattAu/crawlkit/releases/latest/download/crawlkit-linux-amd64 \
      -o /usr/local/bin/crawlkit
    chmod +x /usr/local/bin/crawlkit
```

## Pipeline stages

### 1. Crawl

```bash
crawlkit crawl https://staging.example.com \
  --max-pages 500 \
  --delay 200 \
  --concurrency 8 \
  --output crawl-results/
```

### 2. Validate with custom scripts

```bash
# Fail if any 404s found
sqlite3 crawl-results/crawlkit.db \
  "SELECT COUNT(*) FROM pages WHERE status_code = 404;" | \
  grep -q "^0$" || exit 1

# Fail if average response time > 3 seconds
sqlite3 crawl-results/crawlkit.db \
  "SELECT AVG(load_time_ms) FROM pages WHERE load_time_ms IS NOT NULL;" | \
  awk '{if ($1 > 3000) exit 1}'

# Fail if any page has zero word count (soft 404)
sqlite3 crawl-results/crawlkit.db \
  "SELECT COUNT(*) FROM pages WHERE word_count = 0 AND status_code = 200;" | \
  grep -q "^0$" || exit 1
```

### 3. Compare with baseline

```bash
# Download previous crawl baseline from artifacts
# Then compare:
crawlkit compare baseline/ crawl-results/ --output diff.json

# Fail if new pages were added without titles
jq '.added | length' diff.json | \
  awk '{if ($1 > 0) print "New pages added — verify titles and meta descriptions"}'
```

### 4. Generate and publish report

```bash
crawlkit report crawl-results/ --format html --output seo-report.html
```

## Configuration file for CI

Create `crawlkit-ci.toml`:

```toml
[crawl]
max_pages = 200
delay_ms = 100
concurrency = 8
timeout_secs = 15
user_agent = "crawlkit-ci/1.0"
respect_robots_txt = true

[output]
dir = "crawl-results"
format = "all"
```

```bash
crawlkit crawl https://staging.example.com --config crawlkit-ci.toml
```

## Monitoring regressions

### Trend tracking

Store crawl results across runs and compare:

```yaml
- name: Compare with previous run
  run: |
    # Download previous crawl from artifact storage
    # Compare:
    crawlkit compare previous/ crawl-results/ --output diff.json --format json

    # Alert on regressions
    REMOVED=$(jq '.removed | length' diff.json)
    STATUS_CHANGES=$(jq '.status_changes | length' diff.json)
    echo "Removed: $REMOVED pages, Status changes: $STATUS_CHANGES"

    if [ "$REMOVED" -gt 5 ] || [ "$STATUS_CHANGES" -gt 10 ]; then
      echo "Significant regression detected"
      exit 1
    fi
```

### Slack/Discord notifications

```yaml
- name: Notify on failure
  if: failure()
  run: |
    curl -X POST "$SLACK_WEBHOOK" \
      -H 'Content-type: application/json' \
      -d "{\"text\": \"SEO audit failed for ${{ vars.SITE_URL }}\"}"
```

## Best practices

1. **Crawl your staging environment**, not production, to avoid load impact.
2. **Pin the crawlkit version** to avoid surprise behavior changes.
3. **Set reasonable limits** — `--max-pages 200`, `--delay 100` for CI.
4. **Store artifacts** — keep crawl results for 30 days to track trends.
5. **Use a dedicated bot user-agent** — identify your crawler clearly.
6. **Run on schedule** — daily or weekly crawls catch gradual regressions.
7. **Ignore transient failures** — 502s on staging deploys are expected.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Build timeout | Use pre-built binaries instead of `cargo install` |
| Out of memory | Reduce `--max-pages` or `--concurrency` |
| Rate limited | Increase `--delay` or respect `robots.txt` |
| Staging unreachable | Ensure CI runner has network access to your staging URL |
| SQLite locked | Don't run multiple crawls against the same DB simultaneously |
