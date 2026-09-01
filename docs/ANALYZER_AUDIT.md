# Analyzer Audit — Finding-Code Duplicates & Standards Claims (Phase 3)

Date: 2026-09-01
Scope: `crates/crawlkit-engine/src/analyzers/`

## 1. Finding-code duplicate audit

Static scan of every `pub struct` and the finding `code:` values it emits
(script-derived, reproducible):

- **1,067 distinct finding codes** across all analyzer modules.
- **39 codes are emitted by more than one registered analyzer.**

Consequence: when several generations of the same check fire on one page, the
user sees the "same" issue repeated with an identical code but different
titles/descriptions (e.g. `FORMLBL-V2001` from the deep, deep-deep, and
deep-deep-deep generations). Downstream consumers that key on `code` cannot
deduplicate.

### Cross-module duplicates (highest impact)

| Code | Emitters | Note |
|------|----------|------|
| `COEP001` | `CrossOriginIsolationAnalyzer`, `CrossOriginEmbedderPolicyAnalyzer` | two different analyzers, same code |
| `COOP002` | `CrossOriginIsolationAnalyzer`, `CrossOriginOpenerPolicyAnalyzer` | same |
| `CORS001/002` | `CorsMisconfigurationAnalyzer`, `CorsPolicyAnalyzer` | same |
| `CSPDIR001` | `CspDirectiveValidator`, `CspDirectiveAnalyzer` | same |
| `COOKIESEC001` | `CookieSecureFlagValidator`, `CookieSecurityFlagAnalyzer`, `CookieSecureDeepDeepValidator` | 3 emitters |
| `COOKIEHTTP001` | `CookieHttpOnlyFlagValidator`, `CookieHttpOnlyDeepDeepValidator` | |
| `FOCUS001/002` | `FocusOrderAnalyzer`, `FocusManagementAnalyzer` | |
| `HSTSPR001` | `HstsPreloadReadinessAnalyzer`, `HstsPreloadReadyDeepValidator` | |
| `LINK-V2001` | `LinkAccessibilityAnalyzerV2` (accessibility), `LinkAnalyzerV2` (SEO) | category mismatch under one code |
| `META-V3001` | `MetaDescriptionAnalyzerV3`, `MetaDescriptionLengthAnalyzerV3` | |
| `XCTO-V2001` | `XContentTypeOptionsAnalyzerV2`, `XContentTypeOptionsDeepAnalyzerV2` | |
| `ANCH-DIV001` | `AnchorTextDiversityAnalyzer`, `InternalLinkAnchorTextDiversityAnalyzer` | |

### Intra-module duplicates (`v2_analyzers.rs`, generation stacking)

The deep → deep-deep → deep-deep-deep generations re-emit the parent's code:
`HHIER-V2001/2/3`, `FORMLBL-V2001`, `TABACC-V2001/2`, `IMGALT-V2001`,
`LINKTQ-V2001/2`, `FOCUS-V2001`, `ANCHGEN-V2001`, `FORMLAB-V2001`,
`LANGATTR-V2001`, `COLRCL-V2001`, `COISO-V2001`, `PERMP-V2001`,
`XFODEEP-V2001`, `HSTSPR-V2001`, `INTLINKQ-V2001/2`, `EXTLINKAUTH-V2001`,
`SITEMAPDEEP-V2001`, `ROBOTSDEEP-V2001`, `TBLCAP-V2001`, `TBLSCOP-V2001`.

### Recommended remediation (Phase 4, behavior change — not done here)

1. Namespace codes per generation: `FORMLBL-V2001` (deep) vs
   `FORMLBL-V2001-DD` (deep-deep) etc., **or** drop superseded generations.
2. Give cross-module collisions distinct codes (`COEP001` vs `COI-COEP001`).
3. Add a registry-level test asserting **no two registered analyzers emit the
   same code** (currently only analyzer *names* are asserted unique).

## 2. Standards-claim audit (sampled analyzers)

Verified claims against the actual standards:

### Accurate

- **HSTS (RFC 6797)**: `max-age`, `includeSubDomains`, preload semantics —
  matches the spec; V3 check for `includeSubDomains` presence is correct.
- **X-Content-Type-Options**: `nosniff` is the only valid value — correct.
- **Referrer-Policy**: `strict-origin-when-cross-origin` recommended as the
  modern default — matches current browser default & MDN guidance.
- **Permissions-Policy**: camera/microphone/geolocation directives exist and
  the syntax checked is right.
- **Skip-link / landmark / heading-order checks**: align with WCAG 2.1 AA
  2.4.1 (Bypass Blocks), 1.3.1 (Info and Relationships).
- **Form labels**: missing `<label>`/`aria-label` correctly maps to WCAG
  1.3.1 / 4.1.2 and is a genuine failure.

### Heuristics presented with standards-sounding language (flagged)

- **`HeadingLevelSkipAnalyzer`**: WCAG does **not** hard-fail skipped levels
  (H1→H3); it is a best practice. Finding text is acceptable but severity
  `Warning` is arguably high; keep as advisory.
- **`LandmarkNavAnalyzer` / `LandmarkBannerAnalyzer`**: *missing* nav/banner
  landmarks is **not** a WCAG failure — only `main` (bypass blocks) matters.
  Emitted as `Info` — appropriate; do not raise.
- **Meta description/title length ranges (30–60 / 120–160)**: SEO heuristics,
  not standards. Correctly categorized as SEO, not accessibility — fine.
- **"Multiple H1 headings"**: valid in HTML5; flagged as a heuristic. Titles
  say "Multiple H1 headings" without claiming a WCAG violation — acceptable.

### Conclusion

No analyzer claims a WCAG/OWASP requirement that the standard does not
support. The main Phase 4 work is the finding-code dedup above, plus
(optionally) demoting non-standard heuristics from `Warning` to `Info`.
