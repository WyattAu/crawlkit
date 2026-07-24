# GitHub Actions Integration

This guide shows how to integrate crawlkit into GitHub Actions workflows.

## Basic Crawl Workflow

```yaml
name: SEO Audit
on:
  push:
    branches: [main]

jobs:
  crawl:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install crawlkit
        run: cargo install crawlkit-engine

      - name: Run crawl
        run: |
          crawlkit crawl https://your-site.com \
            --max-pages 100 \
            --output seo-data

      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: seo-report
          path: seo-data/
```

## Scheduled Crawl

```yaml
name: Weekly SEO Audit
on:
  schedule:
    - cron: '0 0 * * 0'  # Every Sunday at midnight

jobs:
  crawl:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install crawlkit
        run: cargo install crawlkit-engine

      - name: Run crawl
        run: |
          crawlkit crawl https://your-site.com \
            --max-pages 500 \
            --seed 42 \
            --output seo-data

      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: weekly-seo-report
          path: seo-data/
```

## Crawl with Encryption

```yaml
- name: Run encrypted crawl
  env:
    CRAWLKIT_ENCRYPTION_KEY: ${{ secrets.CRAWLKIT_ENCRYPTION_KEY }}
  run: |
    crawlkit crawl https://your-site.com \
      --encrypt \
      --output seo-data
```

## Crawl with Authentication

```yaml
- name: Run authenticated crawl
  env:
    JWT_SECRET: ${{ secrets.JWT_SECRET }}
  run: |
    crawlkit-api --port 8080 &
    sleep 5
    crawlkit crawl https://your-site.com \
      --output seo-data
```

## Fail on Critical Issues

```yaml
- name: Check for critical issues
  run: |
    crawlkit crawl https://your-site.com --format json --output results/
    if jq -e '.total_issues > 0' results/crawl-results.json; then
      echo "SEO issues found"
      exit 1
    fi
```
