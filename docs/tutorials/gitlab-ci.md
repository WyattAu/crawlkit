# GitLab CI Integration

This guide shows how to integrate crawlkit into GitLab CI/CD pipelines.

## Basic Crawl Job

```yaml
seo-audit:
  stage: test
  image: rust:latest
  script:
    - cargo install crawlkit-engine
    - crawlkit crawl https://your-site.com --max-pages 100
  artifacts:
    paths:
      - crawlkit.db
    expire_in: 1 week
```

## Scheduled Crawl

```yaml
weekly-seo-audit:
  stage: test
  image: rust:latest
  script:
    - cargo install crawlkit-engine
    - crawlkit crawl https://your-site.com --max-pages 500 --seed 42
  artifacts:
    paths:
      - crawlkit.db
  only:
    - schedules
```

## Crawl with Encryption

```yaml
encrypted-crawl:
  stage: test
  image: rust:latest
  script:
    - cargo install crawlkit-engine
    - crawlkit crawl https://your-site.com --encrypt
  variables:
    CRAWLKIT_ENCRYPTION_KEY: $ENCRYPTION_KEY
  artifacts:
    paths:
      - crawlkit.db
```

## Fail on Critical Issues

```yaml
seo-check:
  stage: test
  image: rust:latest
  script:
    - cargo install crawlkit-engine
    - crawlkit crawl https://your-site.com --format json --output results/
    - |
      if jq -e '.total_issues > 0' results/crawl-results.json; then
        echo "SEO issues found"
        exit 1
      fi
```
