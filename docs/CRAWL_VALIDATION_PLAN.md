# Production Crawl Validation Plan

## Objective
Validate that crawlkit's 840 analyzers produce accurate findings when run against real production websites, and identify systematic false positives that corpus testing cannot catch.

## Target Sites
Select 10 diverse sites covering different:
- Technology stacks (static, SPA, SSR, AMP)
- Industries (e-commerce, media, government, education, tech)
- Sizes (<100 pages, 100-1k, 1k-10k)
- Languages (English, non-English)
- Security postures (strict CSP, no CSP, legacy headers)

### Recommended targets
1. **E-commerce**: https://books.totallyrealistic.shop (or similar small shop)
2. **News/Media**: https://text.npr.org (text-only, good for baseline)
3. **Government**: https://www.gov.uk (accessibility-focused, well-structured)
4. **Documentation**: https://docs.rs (Rust-focused, technical content)
5. **Corporate**: https://about.google.com (large org, rich schema)
6. **SPA**: https://react.dev (React-based, JS-heavy)
7. **Blog**: https://blog.rust-lang.org (simple blog platform)
8. **Education**: https://www.khanacademy.org (educational content)
9. **Non-profit**: https://www.wikipedia.org (massive scale, multi-language)
10. **Developer**: https://github.com (complex, auth-gated content)

## Methodology

### Step 1: Baseline crawl
For each site:
1. Run `crawlkit crawl <url> --max-pages 50 --format json --output ./results/<site-name>/`
2. Record: pages crawled, findings by severity, findings by category
3. Save the JSON output

### Step 2: Manual audit
For each site, manually verify 30 random findings:
1. Open the page in a browser
2. Verify the finding is accurate (TP) or inaccurate (FP)
3. If FP, categorize: wrong logic, missing context, severity too high, duplicate
4. If TP but severity wrong, note the correct severity

### Step 3: Analysis
Compile results:
- Overall FP rate: FPs / total findings
- FP rate by category (which analyzers have the most FPs?)
- FP rate by severity (are Critical findings more accurate than Warnings?)
- Systematic patterns (do certain site types trigger certain FPs?)

### Step 4: Fix
For each systematic FP:
1. Add a corpus fixture that reproduces the FP
2. Fix the analyzer logic
3. Add a regression test
4. Re-run the corpus tests

### Success criteria
- Overall FP rate < 5%
- Critical findings: 0 FPs
- No analyzer with > 20% FP rate
- All FPs have corpus regression tests

## Reporting
Create `docs/benchmarks/validation-report.md` with:
- Per-site results table
- Overall FP rate
- List of FPs found and their categorization
- List of analyzers modified
- Before/after FP rate comparison
