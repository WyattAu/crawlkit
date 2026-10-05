# Finding-Code Catalog

**GENERATED — do not edit by hand.** Regenerate with:

```console
$ python3 scripts/generate_finding_catalog.py
```

The CI gate (`--check`) fails when this file drifts from source or
when a finding code becomes shared by a second analyzer without
being recorded in `KNOWN_SHARED` in
`scripts/generate_finding_catalog.py` (remediation: namespace the
new site per the docs/ANALYZER_AUDIT.md Phase 4 convention, or
consolidate the analyzers).

| Metric | Value |
|---|---|
| Distinct finding codes | 1143 |
| `impl Analyzer for` blocks scanned | 735 |
| Codes shared by 2+ analyzers | 6 |

## Shared codes (recorded ownership)

| Code | Analyzers | Recorded reason |
|---|---|---|
| `COOKIEHTTP001` | `CookieHttpOnlyDeepDeepValidator`, `CookieHttpOnlyFlagValidator` | base CookieHttpOnlyFlagValidator owns the code; the exact-duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported) |
| `COOKIESEC001` | `CookieSecureDeepDeepValidator`, `CookieSecurityFlagAnalyzer` | base CookieSecurityFlagAnalyzer owns the code; the duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported) |
| `SITEMAPDEEP-V2001` | `SitemapCoverageDeepAnalyzerV2`, `SitemapCoverageDeepDeepValidator` | SitemapCoverageDeepAnalyzerV2 owns the code; the exact-duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported) |
| `TBLCAP-V2001` | `TableCaptionPresenceAnalyzerV2`, `TableCaptionPresenceDeepValidator` | TableCaptionPresenceAnalyzerV2 owns the code; the exact-duplicate deep emitter was unregistered 2026-09-27 (impl remains exported) |
| `TBLSCOP-V2001` | `TableHeaderScopeAnalyzerV2`, `TableHeaderScopeDeepValidator` | complementary, not duplicates: TableHeaderScopeAnalyzerV2 fires on <th> elements lacking scope attributes; TableHeaderScopeDeepValidator fires on pages whose tables have no header cells at all (mutually exclusive preconditions) |
| `XFODEEP-V2001` | `XFrameOptionsDeepAnalyzerV2`, `XFrameOptionsDeepDeepValidator` | complementary, not duplicates: XFrameOptionsDeepAnalyzerV2 fires when neither X-Frame-Options nor CSP frame-ancestors is present; XFrameOptionsDeepDeepValidator fires when the header exists but carries an invalid value (mutually exclusive preconditions, pinned by fixture) |

## All codes

| Code | Owner(s) | Source | First title |
|---|---|---|---|
| `A11Y-LINK-V2001` | `LinkAccessibilityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Links with empty text |
| `A11YSC001` | `AccessibilityScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Accessibility compliance score |
| `ACTION-V2001` | `ActionSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/action_schema_v2.rs` | Action schema missing actionType |
| `ACTION001` | `ActionSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/action_schema.rs` | Action schema missing actionType |
| `ACTION002` | `ActionSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/action_schema.rs` | Action schema missing target |
| `AGGOFFER001` | `AggregateOfferSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/aggregate_offer_schema.rs` | AggregateOffer schema missing lowPrice |
| `AGGOFFER002` | `AggregateOfferSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/aggregate_offer_schema.rs` | AggregateOffer schema missing priceCurrency |
| `AI-AB001` | `AiAnswerBoxAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | No FAQ schema detected |
| `AI-AB003` | `AiAnswerBoxAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | No question/answer format detected |
| `AI-ACC009` | `AiCrawlerAccessibilityAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | No robots.txt found |
| `AI-CIT001` | `AiCitationEligibilityAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | Missing canonical URL |
| `AI-CIT005` | `AiCitationEligibilityAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | No structured data found |
| `AI-CIT007` | `AiCitationEligibilityAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | Missing OpenGraph tags |
| `AI-CS002` | `AiContentStructureAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | Content lacks subheadings |
| `AI-CS008` | `AiContentStructureAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | Missing date metadata |
| `AI-CS009` | `AiContentStructureAnalyzer` | `crates/crawlkit-engine/src/ai_analyzers.rs` | Missing author attribution |
| `ANCH-DIV001` | `AnchorTextDiversityAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | All internal links use identical anchor text |
| `ANCH-DIV002` | `AnchorTextDiversityAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Overuse of generic anchor text |
| `ANCH-V3001` | `InternalLinkAnchorAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Generic internal anchor text |
| `ANCHGEN-V2001` | `AnchorTextGenericAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Generic anchor text |
| `ANCHGEN-V2001-DEEP` | `AnchorTextGenericDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Generic anchor text (deep) |
| `ANCHGEN001` | `AnchorTextGenericAnalyzer` | `crates/crawlkit-engine/src/analyzers/aria_focus_link_validator_analyzers.rs` | Link with generic anchor text |
| `ANCHOR001` | `InternalLinkAnchorAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Anchor text identical to URL |
| `ANCHOR002` | `InternalLinkAnchorAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Over-optimized anchor text |
| `APT-V2001` | `ApartmentSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/apartment_schema_v2.rs` | Apartment schema missing numberOfRooms |
| `APT001` | `ApartmentSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/apartment_schema.rs` | Apartment schema missing name |
| `APT002` | `ApartmentSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/apartment_schema.rs` | Apartment schema missing numberOfRooms |
| `APTROOM-V2001` | `ApartmentMissingNumberOfRoomsValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Apartment missing numberOfRooms |
| `APTROOM001` | `ApartmentMissingNumberOfRoomsValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Apartment missing numberOfRooms |
| `ARAT001` | `AggregateRatingValidator` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | AggregateRating ratingValue exceeds bestRating |
| `ARAT002` | `AggregateRatingValidator` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | AggregateRating reviewCount or ratingCount is 0 |
| `ARIA-V2001` | `AriaRolesAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | ARIA roles without names |
| `ARIA001` | `AriaRolesAnalyzer` | `crates/crawlkit-engine/src/analyzers/aria_analyzers.rs` | ARIA roles without accessible names |
| `ARIA002` | `AriaRolesAnalyzer` | `crates/crawlkit-engine/src/analyzers/aria_analyzers.rs` | ARIA roles may need accessible names on non-semantic elements |
| `ARIALABEL001` | `AriaLabelAnalyzer` | `crates/crawlkit-engine/src/analyzers/aria_label_analyzers.rs` | ARIA roles without labels |
| `ARIALAND-ROLE-BANNER` | `AriaLandmarksAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing banner landmark |
| `ARIALAND-ROLE-CONTENTINFO` | `AriaLandmarksAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing contentinfo landmark |
| `ARIALAND-ROLE-MAIN` | `AriaLandmarksAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing main landmark |
| `ARIALAND-ROLE-NAVIGATION` | `AriaLandmarksAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing navigation landmark |
| `ARIALAND-V2001` | `AriaLandmarksDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No ARIA landmarks found (deep) |
| `ARIALAND-V2002` | `AriaLandmarksDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing main landmark (deep) |
| `ARIALAND-V2003` | `AriaLandmarksDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing navigation landmark (deep) |
| `ARIALAND-V2005` | `AriaLandmarksAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Duplicate main landmarks |
| `ARIALAND-V2006` | `AriaLandmarksAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No ARIA landmarks found |
| `ARIALAND001` | `AriaLandmarksAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Missing main landmark |
| `ARIALAND002` | `AriaLandmarksAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Missing navigation landmark |
| `ARIALAND003` | `AriaLandmarksAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | (no title literal) |
| `ARIAREQ-V2001` | `AriaRequiredAttributesDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | ARIA roles without labels (deep) |
| `ARIAREQ-V2002` | `AriaRequiredAttributesAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | progressbar missing aria-valuenow |
| `ARIAREQ-V2003` | `AriaRequiredAttributesAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | slider missing required attributes |
| `ARIAREQ-V2004` | `AriaRequiredAttributesAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | (no title literal) |
| `ARIAREQ-V2005` | `AriaRequiredAttributesAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | tab missing aria-selected |
| `ARIAREQ001` | `AriaRequiredAttributesAnalyzer` | `crates/crawlkit-engine/src/analyzers/aria_focus_link_validator_analyzers.rs` | ARIA roles missing required accessible name attributes |
| `ARIAROLE001` | `AriaRoleAnalyzer` | `crates/crawlkit-engine/src/analyzers/basic_accessibility_analyzers.rs` | ARIA roles without accessible names |
| `ART001` | `ArticleSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/article_schema.rs` | Article schema missing headline |
| `ART002` | `ArticleSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/article_schema.rs` | Article schema missing datePublished |
| `ART003` | `ArticleSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/article_schema.rs` | Article schema missing author |
| `ARTAUTH-V2001` | `ArticleMissingAuthorValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Article missing author |
| `ARTAUTH-V5001` | `ArticleAuthorUrlValidator` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Author missing URL |
| `ARTDT-V2001` | `ArticleMissingDatePublishedValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Article missing datePublished |
| `ARTHL-V2001` | `ArticleMissingHeadlineValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Article missing headline |
| `ARTMOD-V5001` | `ArticleDateModifiedValidator` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Article missing dateModified |
| `ARTWC-V5001` | `ArticleWordCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Article too short |
| `ASYNC001` | `AsyncScriptAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Render-blocking scripts without async/defer |
| `ASYNC002` | `AsyncScriptAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Inline scripts may block rendering |
| `AVAIL001` | `OfferAvailabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/schema/offer_availability.rs` | Schema says InStock but page says out of stock |
| `AVAIL002` | `OfferAvailabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/schema/offer_availability.rs` | Schema says OutOfStock but page says in stock |
| `BDEPTH001` | `BreadcrumbListDepthAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Breadcrumb depth inconsistent with URL depth |
| `BENAME-V6020` | `BroadcastEventMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | BroadcastEvent missing name |
| `BOOK001` | `BookSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/book_schema_v2.rs` | Book schema missing isbn |
| `BOOKAUTH-V2001` | `BookMissingAuthorValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing author |
| `BOOKAUTH-V6024` | `BookMissingAuthorValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing author |
| `BOOKDATE-V2001` | `BookMissingDatePublishedValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing datePublished |
| `BOOKDATE-V6026` | `BookMissingDatePublishedValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing datePublished |
| `BOOKFMT001` | `BookMissingBookFormatValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing bookFormat |
| `BOOKISBN-V2001` | `BookMissingIsbnValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing ISBN |
| `BOOKISBN-V6025` | `BookMissingIsbnValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing isbn |
| `BOOKPUB-V6055` | `BookMissingPublisherValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Book missing publisher |
| `BRAND001` | `BrandSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/brand_schema.rs` | Brand schema missing name |
| `BRAND002` | `BrandSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/brand_schema.rs` | Brand schema missing url |
| `BREAD001` | `BreadcrumbsValidator` | `crates/crawlkit-engine/src/analyzers/schema/breadcrumbs.rs` | BreadcrumbList has too few items |
| `BREAD002` | `BreadcrumbsValidator` | `crates/crawlkit-engine/src/analyzers/schema/breadcrumbs.rs` | Breadcrumb URL doesn't match page URL |
| `BREAD003` | `BreadcrumbsValidator` | `crates/crawlkit-engine/src/analyzers/schema/breadcrumbs.rs` | Deep page missing BreadcrumbList schema |
| `BREADCRITM-V5001` | `BreadcrumbItemCountValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Breadcrumb has too few items |
| `BREADURL-V5001` | `BreadcrumbUrlValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Breadcrumb has relative URL |
| `BROADCAST001` | `BroadcastEventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/broadcast_event_schema.rs` | BroadcastEvent schema missing name |
| `BROADCAST002` | `BroadcastEventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/broadcast_event_schema.rs` | BroadcastEvent schema missing dates |
| `BROADCAST003` | `BroadcastEventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/broadcast_event_schema.rs` | BroadcastEvent schema missing broadcastOfEvent |
| `CACHE001` | `CacheHeaderAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Missing Cache-Control header |
| `CACHE002` | `CacheHeaderAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | No ETag or Last-Modified header |
| `CACHE003` | `CacheHeaderAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | HTML content marked as non-cacheable |
| `CAN-V3001` | `CanonicalUrlAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing canonical URL |
| `CAN-V3002` | `CanonicalUrlAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Empty canonical URL |
| `CAN-V3005` | `CanonicalUrlAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical has fragment |
| `CANCH001` | `CanonicalChainDetector` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Canonical URL points to different domain (possible chain) |
| `CANCH002` | `CanonicalChainDetector` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Canonical points to non-indexable page |
| `CANCHAIN-V2001` | `CanonicalChainDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Multiple canonical tags (deep-deep-deep) |
| `CANCHAIN-V5001` | `CanonicalChainValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical points off-page |
| `CANCHAIN-V6090` | `CanonicalChainDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Multiple canonical tags |
| `CANCHAIN001` | `CanonicalChainDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Multiple canonical tags |
| `CANDEEP-V2001` | `CanonicalDepthDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical URL very deep (deep-deep) |
| `CANDEEP-V2005` | `CanonicalValidationDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical contains fragment |
| `CANDEEP-V2006` | `CanonicalValidationDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical has query params |
| `CANDEEP-V6091` | `CanonicalDepthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical URL path deeply nested |
| `CANDEEP001` | `CanonicalValidationDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical path mismatch |
| `CANDEEP002` | `CanonicalValidationDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical differs by parameters |
| `CANDEEP003` | `CanonicalValidationDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical URL contains fragment |
| `CANDEPTH-V5001` | `CanonicalDepthValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical URL too deep |
| `CANMISS-V2001` | `CanonicalMissingDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing canonical URL (deep-deep) |
| `CANMISS-V6088` | `CanonicalMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing canonical URL |
| `CANMISS001` | `CanonicalMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing canonical on paginated page |
| `CANON001` | `CanonicalUrlValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing canonical URL |
| `CANON002` | `CanonicalUrlValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical URL differs |
| `CANON003` | `CanonicalUrlValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical URL mismatch |
| `CANON004` | `AdvancedCanonicalAnalyzer` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Canonical may point to redirect |
| `CANSELF-V5001` | `CanonicalSelfReferenceValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical points to different URL |
| `CANSELFRF-V2001` | `CanonicalSelfReferenceDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical has different scheme/host (deep-deep-deep) |
| `CANSELFRF-V6089` | `CanonicalSelfReferenceDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical does not self-reference |
| `CANSELFRF001` | `CanonicalSelfReferenceDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Canonical has different scheme/host |
| `CAR-V2001` | `CarSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/car_schema_v2.rs` | Car schema missing model |
| `CAR001` | `CarSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/car_schema.rs` | Car schema missing name |
| `CAR002` | `CarSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/car_schema.rs` | Car schema missing model |
| `CAR003` | `CarSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/car_schema.rs` | Car schema missing manufacturer |
| `CARMFR001` | `CarMissingManufacturerValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Car missing manufacturer |
| `CARMODEL-V2001` | `CarMissingModelValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Car missing model |
| `CARMODEL001` | `CarMissingModelValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Car missing model |
| `CDEP001` | `CanonicalDepthAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical URL is deeply nested |
| `CDEP002` | `CanonicalDepthAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical URL contains query parameters |
| `CHARSET001` | `CharsetValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing charset declaration |
| `CHARSET002` | `CharsetValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Charset missing from HTTP header |
| `CHARSET003` | `CharsetValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Non-UTF-8 charset |
| `CIVIC001` | `CivicStructureSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/civic_structure_schema.rs` | CivicStructure schema missing name |
| `CIVIC002` | `CivicStructureSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/civic_structure_schema.rs` | CivicStructure schema missing location |
| `COEP-V2001` | `CrossOriginIsolationAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | Missing Cross-Origin-Embedder-Policy header |
| `COEP-V5001` | `CoepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Cross-Origin-Embedder-Policy |
| `COEP001-ISOLATION` | `CrossOriginIsolationAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Missing Cross-Origin-Embedder-Policy header |
| `COEP001-POLICY` | `CrossOriginEmbedderPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Cross-Origin-Embedder-Policy header missing |
| `COEP002` | `CrossOriginEmbedderPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Cross-Origin-Embedder-Policy not set to require-corp |
| `COEPU-V6076` | `CoepRequireCorpValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | COEP set to unsafe-none |
| `COISO-V2001` | `CrossOriginIsolationDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing COEP |
| `COISO-V2001-DEEP-DEEP` | `CrossOriginIsolationDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Cross-Origin-Embedder-Policy (deep-deep) |
| `COISO-V2002` | `CrossOriginIsolationDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Cross-Origin-Opener-Policy (deep-deep) |
| `COISO-V2003` | `CrossOriginIsolationDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing COOP |
| `COISODEEP001` | `CrossOriginIsolationDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Missing Cross-Origin-Embedder-Policy |
| `COISODEEP002` | `CrossOriginIsolationDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Missing Cross-Origin-Opener-Policy |
| `COISODEEP003` | `CrossOriginIsolationDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Partial cross-origin isolation |
| `COLRCL-V2001-DEEP` | `ColorContrastLinkDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Possible link contrast issue (deep) |
| `COLRCL001` | `ColorContrastLinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/csp_color_contrast_analyzers.rs` | Link color contrast too low |
| `COLRCT-V2001` | `ColorContrastTextDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Possible light-on-light contrast issue (deep) |
| `COLRCT-V2003` | `ColorContrastTextAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Hidden text detected |
| `COLRCT001` | `ColorContrastTextAnalyzer` | `crates/crawlkit-engine/src/analyzers/csp_color_contrast_analyzers.rs` | Low text color contrast ratio |
| `COMP001` | `CompressionAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Large response not compressed |
| `COMP002` | `CompressionAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Unnecessary compression for small response |
| `CONN001` | `ConnectionAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Too many unique external domains |
| `CONN002` | `ConnectionAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Missing preconnect for external origins |
| `CONTR001` | `ColorContrastAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_visual_analyzers.rs` | Text color too similar to background color |
| `CONTR002` | `ColorContrastAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_visual_analyzers.rs` | Low color contrast ratio (below 4.5:1) |
| `COOKIE-V2002` | `CookieSecurityFlagAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIE-V2003` | `CookieSecurityFlagAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIE-V2004` | `CookieSecurityFlagAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIE001` | `CookieAnalyzer` | `crates/crawlkit-engine/src/analyzers/cookies.rs` | Cookie missing Secure flag |
| `COOKIE002` | `CookieAnalyzer` | `crates/crawlkit-engine/src/analyzers/cookies.rs` | Cookie missing HttpOnly flag |
| `COOKIEHTTP-V2001` | `CookieHttpOnlyDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIEHTTP-V5001` | `CookieHttpOnlyFlagValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIEHTTP-V6066` | `CookieHttpOnlyFlagDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIEHTTP001` | `CookieHttpOnlyDeepDeepValidator`, `CookieHttpOnlyFlagValidator` | `crates/crawlkit-engine/src/analyzers/cookies.rs`, `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESAME-V2001` | `CookieSameSiteDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESAME-V5001` | `CookieSameSiteValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESAME-V6067` | `CookieSameSiteDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESAME001` | `CookieSameSiteDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESEC-V2001` | `CookieSecureDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESEC-V5001` | `CookieSecureFlagValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESEC-V6065` | `CookieSecureFlagDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESEC001` | `CookieSecureDeepDeepValidator`, `CookieSecurityFlagAnalyzer` | `crates/crawlkit-engine/src/analyzers/cookies.rs`, `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `COOKIESEC001-VALIDATOR` | `CookieSecureFlagValidator` | `crates/crawlkit-engine/src/analyzers/cookies.rs` | Cookie missing Secure flag |
| `COOKIESEC002` | `CookieSecurityFlagAnalyzer` | `crates/crawlkit-engine/src/analyzers/cookies.rs` | (no title literal) |
| `COOKIESEC003` | `CookieSecurityFlagAnalyzer` | `crates/crawlkit-engine/src/analyzers/cookies.rs` | (no title literal) |
| `COOP-V2001` | `CrossOriginOpenerPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | Missing Cross-Origin-Opener-Policy header |
| `COOP-V5001` | `CoopValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Cross-Origin-Opener-Policy |
| `COOP001` | `CrossOriginOpenerPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Cross-Origin-Opener-Policy header missing |
| `COOP002-ISOLATION` | `CrossOriginIsolationAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Missing Cross-Origin-Opener-Policy header |
| `COOP002-POLICY` | `CrossOriginOpenerPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Cross-Origin-Opener-Policy not set to same-origin |
| `COOPU-V6077` | `CoopSameOriginValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | COOP set to unsafe-none |
| `CORP001` | `CrossOriginResourcePolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/x_header_analyzers.rs` | Missing Cross-Origin-Resource-Policy header |
| `CORS-V2001` | `CorsPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CORS wildcard with credentials |
| `CORS-V2003` | `CorsPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CORS origin differs from page |
| `CORS001` | `CorsPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | CORS wildcard with credentials |
| `CORS001-MISCONFIG` | `CorsMisconfigurationAnalyzer` | `crates/crawlkit-engine/src/analyzers/dns_sri_cors_analyzers.rs` | CORS allows all origins with credentials |
| `CORS002` | `CorsPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | CORS allows all origins |
| `CORS002-MISCONFIG` | `CorsMisconfigurationAnalyzer` | `crates/crawlkit-engine/src/analyzers/dns_sri_cors_analyzers.rs` | CORS wildcard on sensitive endpoint |
| `CORSWILD-V2001` | `CorsWildcardDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CORS wildcard with credentials (deep-deep) |
| `CORSWILD-V6069` | `CorsWildcardValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CORS allows all origins |
| `CORSWILD001` | `CorsWildcardDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CORS wildcard with credentials |
| `COUP001` | `CouponSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/coupon_schema.rs` | Coupon schema missing validFrom |
| `COUP002` | `CouponSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/coupon_schema.rs` | Coupon schema missing discount information |
| `COURSE001` | `CourseSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/course_schema.rs` | Course schema missing name |
| `COURSE002` | `CourseSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/course_schema.rs` | Course schema missing provider |
| `COURSEDESC-V5001` | `CourseDescriptionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Course missing description |
| `COURSENAME-V2001` | `CourseMissingNameValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Course missing name |
| `COURSEPRE001` | `CoursePrerequisiteValidator` | `crates/crawlkit-engine/src/analyzers/schema/course_prerequisite.rs` | Course missing prerequisites |
| `COURSEPV-V5001` | `CourseProviderNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Course provider missing name |
| `CPROV001` | `CourseProviderValidator` | `crates/crawlkit-engine/src/analyzers/schema/course_provider.rs` | Course provider missing name |
| `CPROV002` | `CourseProviderValidator` | `crates/crawlkit-engine/src/analyzers/schema/course_provider.rs` | Course provider missing URL |
| `CQ001` | `ContentQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Flesch-Kincaid readability score |
| `CQ002` | `ContentQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Top keywords |
| `CQ003` | `ContentQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | No content detected |
| `CQ004` | `ContentQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Thin content |
| `CQ005` | `ContentQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Long-form content |
| `CREATIVE001` | `CreativeWorkSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/creative_work_schema.rs` | CreativeWork schema missing name |
| `CREATIVE002` | `CreativeWorkSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/creative_work_schema.rs` | CreativeWork schema missing author |
| `CREATIVE003` | `CreativeWorkSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/creative_work_schema.rs` | CreativeWork schema missing date |
| `CSP-V2001` | `ContentSecurityPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | CSP missing script-src directive |
| `CSP001` | `ContentSecurityPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | CSP script-src allows unsafe-inline |
| `CSP002` | `ContentSecurityPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | CSP missing frame-ancestors directive |
| `CSPBASE-V2001` | `CspBaseUriSelfDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP base-uri missing 'self' (deep-deep) |
| `CSPBASE-V6079` | `CspBaseUriSelfValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing base-uri |
| `CSPBASE001` | `CspBaseUriSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP base-uri missing 'self' |
| `CSPCON-V2001` | `CspConnectSrcDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP connect-src allows wildcard (deep) |
| `CSPCON001` | `CspConnectSrcAnalysisValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP connect-src allows wildcard |
| `CSPDIR-V2001` | `CspDirectiveAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing base-uri |
| `CSPDIR-V2002` | `CspDirectiveAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing form-action |
| `CSPDIR-V2003` | `CspDirectiveAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing frame-ancestors |
| `CSPDIR-V2004` | `CspDirectiveAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing object-src |
| `CSPDIR001` | `CspDirectiveAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | CSP missing default-src |
| `CSPDIR001-VALIDATOR` | `CspDirectiveValidator` | `crates/crawlkit-engine/src/analyzers/csp_color_contrast_analyzers.rs` | (no title literal) |
| `CSPDIR002` | `CspDirectiveAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | (no title literal) |
| `CSPDIR003` | `CspDirectiveAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | (no title literal) |
| `CSPEXEC-V6061` | `CspConnectSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing connect-src |
| `CSPFNT-V2001` | `CspFontSrcDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP font-src allows wildcard (deep) |
| `CSPFNT001` | `CspFontSrcAnalysisValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP font-src allows wildcard |
| `CSPFONT-V6062` | `CspFontSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing font-src |
| `CSPFORM-V2001` | `CspFormActionSelfDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP form-action missing 'self' (deep-deep) |
| `CSPFORM-V6080` | `CspFormActionSelfValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing form-action |
| `CSPFORM001` | `CspFormActionSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP form-action missing 'self' |
| `CSPFRAME-V5001` | `CspFrameAncestorsValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing frame-ancestors |
| `CSPOBJ-V2001` | `CspObjectSrcNoneDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP object-src not set to 'none' (deep-deep) |
| `CSPOBJ-V6078` | `CspObjectSrcNoneValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP object-src not set to 'none' |
| `CSPOBJ001` | `CspObjectSrcNoneDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP object-src not set to 'none' |
| `CSPSS-V2001` | `CspScriptSrcSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src missing 'self' (deep) |
| `CSPSS-V2002` | `CspScriptSrcSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src allows unsafe-inline |
| `CSPSS-V2003` | `CspScriptSrcSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src allows unsafe-eval |
| `CSPSS001` | `CspScriptSrcSelfValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src missing 'self' |
| `CSPSSRC-V5001` | `CspScriptSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src allows unsafe-inline |
| `CSPSSRC-V5002` | `CspScriptSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src allows unsafe-eval |
| `CSPSSRC-V5003` | `CspScriptSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP script-src wildcard |
| `CSPSSRC-V5004` | `CspScriptSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing script-src |
| `CSPSTY-V2001` | `CspStyleSrcSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP style-src missing 'self' (deep) |
| `CSPSTY-V2002` | `CspStyleSrcSelfDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP style-src allows unsafe-inline |
| `CSPSTY001` | `CspStyleSrcSelfValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP style-src missing 'self' |
| `CSPSTYLE-V5001` | `CspStyleSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP style-src allows unsafe-inline |
| `CSPSTYLE-V5002` | `CspStyleSrcValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | CSP missing style-src |
| `CT-V2001` | `ContentTypeSniffingAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | Missing X-Content-Type-Options header |
| `CT-V2002` | `ContentTypeSniffingAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | Invalid X-Content-Type-Options value |
| `CTSNIFF001` | `ContentTypeSniffingAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Missing X-Content-Type-Options header |
| `CTSNIFF002` | `ContentTypeSniffingAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | X-Content-Type-Options not set to nosniff |
| `CVNAME-V6010` | `CivicStructureMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | CivicStructure missing name |
| `CWDATE-V6003` | `CreativeWorkMissingDateCreatedValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | CreativeWork missing dateCreated |
| `CWDESC-V6002` | `CreativeWorkMissingDescriptionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | CreativeWork missing description |
| `CWLIC-V6053` | `CreativeWorkMissingLicenseValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | CreativeWork missing license |
| `CWNAME-V6001` | `CreativeWorkMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | CreativeWork missing name |
| `DATA001` | `DatasetSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/dataset_schema.rs` | Dataset schema missing name |
| `DATA002` | `DatasetSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/dataset_schema.rs` | Dataset schema missing distribution |
| `DATDESC001` | `DatasetMissingDescriptionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Dataset missing description |
| `DATDIST001` | `DatasetMissingDistributionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Dataset missing distribution |
| `DNSPREFETCH-V5001` | `DnsPrefetchHintValidator` | `crates/crawlkit-engine/src/analyzers/v2/performance.rs` | No dns-prefetch hints |
| `DNSREBIND001` | `DnsRebindingAnalyzer` | `crates/crawlkit-engine/src/analyzers/dns_sri_cors_analyzers.rs` | CORS wildcard with credentials enabled |
| `DNSREBIND002` | `DnsRebindingAnalyzer` | `crates/crawlkit-engine/src/analyzers/dns_sri_cors_analyzers.rs` | CORS wildcard with local network references |
| `DSDIST-V5001` | `DatasetDistributionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Dataset distribution missing contentUrl |
| `DSLICENSE-V5001` | `DatasetLicenseValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Dataset missing license |
| `DUMMY001` | `DummyAnalyzer` | `crates/crawlkit-engine/src/analyzers/tests/test_main.rs` | Dummy finding |
| `DUP-V2001` | `DuplicateContentDetectorV2` | `crates/crawlkit-engine/src/analyzers/v2/content.rs` | Repeated text chunks detected |
| `DUP001` | `DuplicateContentDetector` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Title and description are nearly identical |
| `DUP002` | `DuplicateContentDetector` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Description starts with title text |
| `DUP003` | `DuplicateContentDetector` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Low content diversity detected |
| `ECOM001` | `EcommerceSignalsAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Product schema detected |
| `ECOM002` | `EcommerceSignalsAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Price information detected |
| `ECOM003` | `EcommerceSignalsAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Availability information detected |
| `ECOM004` | `EcommerceSignalsAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Review/rating information detected |
| `ECOM005` | `EcommerceSignalsAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Offer schema detected |
| `ECOM006` | `EcommerceSignalsAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Price data without Product schema |
| `EDUNAME-V6017` | `EducationalOrganizationMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | EducationalOrganization missing name |
| `EDUORG001` | `EducationalOrganizationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/educational_organization_schema.rs` | EducationalOrganization schema missing name |
| `EDUORG002` | `EducationalOrganizationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/educational_organization_schema.rs` | EducationalOrganization schema missing address |
| `EDUORG003` | `EducationalOrganizationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/educational_organization_schema.rs` | EducationalOrganization schema missing url |
| `ELOC001` | `EventLocationValidator` | `crates/crawlkit-engine/src/analyzers/schema/event_location.rs` | Event missing location |
| `ELOC002` | `EventLocationValidator` | `crates/crawlkit-engine/src/analyzers/schema/event_location.rs` | Event location missing name |
| `ENTITY001` | `EntityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | People entities detected |
| `ENTITY002` | `EntityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Organization entities detected |
| `ENTITY003` | `EntityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Location entities detected |
| `ENTITY004` | `EntityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Detected topics and themes |
| `ENTITY005` | `EntityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Content sentiment analysis |
| `ENTITY006` | `EntityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Entity counts per page |
| `EVENT001` | `EventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/event_schema.rs` | Event schema missing startDate |
| `EVENT002` | `EventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/event_schema.rs` | Event schema missing location |
| `EVENT003` | `EventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/event_schema.rs` | Event schema missing organizer |
| `EVENTPAST001` | `EventStartDateValidator` | `crates/crawlkit-engine/src/analyzers/schema/event_start_date.rs` | Event has a past startDate |
| `EVTLOC-V6059` | `EventMissingLocationValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Event missing location |
| `EVTORG-V5001` | `EventOrganizerValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Event organizer missing name |
| `EVTORG-V5002` | `EventOrganizerValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Event missing organizer |
| `EVTPERF-V5001` | `EventPerformerValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Event missing performer |
| `EVTSTART-V6058` | `EventMissingStartDateValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Event missing startDate |
| `EXTAUTH-V6105` | `ExternalLinksAuthorityScoreValidator` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | No authoritative external links |
| `EXTAUTHDP-V2001` | `ExternalLinksAuthorityScoreDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Low authority link ratio (deep-deep) |
| `EXTAUTHDP001` | `ExternalLinksAuthorityScoreDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | No high-authority external links |
| `EXTAUTHDP002` | `ExternalLinksAuthorityScoreDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Links to low-reputation TLDs |
| `EXTAUTHDP003` | `ExternalLinksAuthorityScoreDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Low authority link ratio |
| `EXTLINKAUTH-V2001` | `ExternalLinkAuthorityDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Links to suspicious TLDs |
| `EXTLINKAUTH-V2001-DEEP-DEEP` | `ExternalLinkAuthorityDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Many low-authority external links (deep-deep) |
| `EXTLINKAUTH001` | `ExternalLinkAuthorityDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Many followed external links |
| `EXTLINKAUTH002` | `ExternalLinkAuthorityDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | External links with empty anchor text |
| `EXTLINKAUTH003` | `ExternalLinkAuthorityDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | External links pointing to same domain |
| `EXTNOFOLLOW-V6106` | `ExternalLinksNofollowAnalysisValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No nofollow on external links |
| `FAQ001` | `FaqSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/faq_schema.rs` | FAQPage schema missing mainEntity |
| `FAQ002` | `FaqSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/faq_schema.rs` | FAQPage schema has fewer than 2 questions |
| `FAQ003` | `FaqSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/faq_schema.rs` | FAQPage question missing acceptedAnswer |
| `FAQANSLEN-V5001` | `FAQAnswerLengthValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | FAQ answers too short |
| `FAQPAGE001` | `FaqPageEntityValidator` | `crates/crawlkit-engine/src/analyzers/schema/faq_page_entity.rs` | FAQPage mainEntity is not an array |
| `FECUIS-V6006` | `FoodEstablishmentMissingServesCuisineValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | FoodEstablishment missing servesCuisine |
| `FEMENU-V6005` | `FoodEstablishmentMissingMenuValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | FoodEstablishment missing menu |
| `FILABEL001` | `FormInputLabelAnalyzer` | `crates/crawlkit-engine/src/analyzers/basic_accessibility_analyzers.rs` | Form input missing associated label |
| `FLABEL001` | `FormLabelAnalyzer` | `crates/crawlkit-engine/src/analyzers/form_analyzers.rs` | Form input missing associated label |
| `FLABEL002` | `FormLabelAnalyzer` | `crates/crawlkit-engine/src/analyzers/form_analyzers.rs` | Form input with empty label text |
| `FOCTR001` | `FocusTrapMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Dialog without aria-modal |
| `FOCUS-V2001` | `FocusManagementDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Positive tabindex found |
| `FOCUS-V2001-DEEP-DEEP` | `FocusManagementDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Positive tabindex found (deep-deep) |
| `FOCUS-V2001-DEEP-DEEP-DEEP` | `FocusManagementDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Positive tabindex found (deep-deep-deep) |
| `FOCUS001` | `FocusManagementAnalyzer` | `crates/crawlkit-engine/src/analyzers/focus_management_analyzers.rs` | Positive tabindex values disrupt focus order |
| `FOCUS002` | `FocusManagementAnalyzer` | `crates/crawlkit-engine/src/analyzers/focus_management_analyzers.rs` | No visible focus indicators found |
| `FOCUSDEEP001` | `FocusManagementDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Positive tabindex detected |
| `FOCUSDEEP002` | `FocusManagementDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Many elements with tabindex=-1 |
| `FOCUSTABPOS-V6119` | `FocusTabindexPositiveValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Positive tabindex detected |
| `FONT001` | `FontDisplayAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Web fonts missing font-display:swap |
| `FONT002` | `FontDisplayAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Multiple font files loaded |
| `FOOD001` | `FoodEstablishmentSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/food_establishment_schema.rs` | FoodEstablishment schema missing name |
| `FOOD002` | `FoodEstablishmentSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/food_establishment_schema.rs` | FoodEstablishment schema missing address |
| `FOOD003` | `FoodEstablishmentSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/food_establishment_schema.rs` | FoodEstablishment schema missing servesCuisine |
| `FORM-V2001` | `FormAccessibilityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Forms without labels |
| `FORM001` | `FormAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Form missing action URL |
| `FORMFSLG-V6123` | `FormFieldsetLegendValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Complex forms without fieldset |
| `FORMLAB-V2001` | `FormLabelAssociationAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Duplicate input IDs |
| `FORMLAB-V2001-DEEP` | `FormLabelAssociationDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Form inputs without label association (deep) |
| `FORMLAB001` | `FormLabelAssociationAnalyzer` | `crates/crawlkit-engine/src/analyzers/form_table_validator_analyzers.rs` | Form inputs missing label associations |
| `FORMLBL-V2001` | `FormLabelsDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Form elements without labels |
| `FORMLBL-V2001-DEEP-DEEP` | `FormLabelsDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Form inputs without labels (deep-deep) |
| `FORMLBL-V2001-DEEP-DEEP-DEEP` | `FormLabelsDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Form inputs without labels (deep-deep-deep) |
| `FORMLBLASSOC-V5001` | `FormLabelAssociationValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Form inputs without labels |
| `FORMLBLDEEP001` | `FormLabelsDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Forms present but no ARIA labels |
| `FORMLBLDEEP002` | `FormLabelsDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Insufficient ARIA labels for forms |
| `FORMREQ-V5001` | `FormRequiredFieldsValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Required fields missing labels |
| `FORMREQ-V6110` | `FormRequiredFieldsDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Required fields missing labels |
| `FP001` | `FeaturePolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | No Feature-Policy or Permissions-Policy header |
| `FRESH001` | `ContentFreshnessScorer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | No date metadata on time-sensitive content |
| `FRESH002` | `ContentFreshnessScorer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Date mismatch between visible text \
                                                             and schema |
| `FRESHSC001` | `ContentFreshnessScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | No date metadata found |
| `FRESHSC002` | `ContentFreshnessScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Content is over a year old |
| `FRESHSC003` | `ContentFreshnessScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Content is over 6 months old |
| `GOV-V2001` | `GovernmentServiceSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/government_service_schema_v2.rs` | GovernmentService schema missing provider |
| `GOV001` | `GovernmentServiceSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/government_service_schema.rs` | GovernmentService schema missing name |
| `GOV002` | `GovernmentServiceSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/government_service_schema.rs` | GovernmentService schema missing provider |
| `GOVSERVICE001` | `GovernmentServiceMissingServiceAreaValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | GovernmentService missing serviceArea |
| `H1COUNT-V6107` | `HeadingH1CountValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing H1 heading |
| `H1MULTI-V6108` | `HeadingH1CountValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Multiple H1 headings |
| `HEAD-V2001` | `HeadingHierarchyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Heading levels skip |
| `HEAD001` | `HeadingHierarchyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | No headings found |
| `HEAD002` | `HeadingHierarchyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing H1 heading |
| `HEAD003` | `HeadingHierarchyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Multiple H1 headings |
| `HEAD004` | `HeadingHierarchyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Skipped heading level |
| `HEAD005` | `HeadingHierarchyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Deep heading hierarchy |
| `HEADDEPTH-V6109` | `HeadingDepthAnalysisValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Headings reach H6 level |
| `HEADEMPTY-V6122` | `HeadingEmptyValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Empty headings |
| `HEADH1-V5001` | `HeadingMultipleH1Validator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing H1 heading |
| `HEADH1-V5002` | `HeadingMultipleH1Validator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Multiple H1 headings |
| `HEADSC001` | `HeadingStructureScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | No headings found |
| `HEADSC002` | `HeadingStructureScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Missing H1 heading |
| `HEADSC003` | `HeadingStructureScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Multiple H1 headings |
| `HEADSC004` | `HeadingStructureScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Heading level skipped |
| `HEADSKIP-V2001` | `HeadingSkipLevelsDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Heading levels skipped (deep-deep) |
| `HEADSKIP-V5001` | `HeadingSkipLevelsValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Heading levels skipped |
| `HEADSKIP-V6121` | `HeadingSkipLevelsDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Heading levels skipped |
| `HEADSKIP001` | `HeadingLevelSkipAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_heading_validator_analyzers.rs` | Heading level skip detected |
| `HEALTHY-001` | `HealthyAnalyzer` | `crates/crawlkit-engine/src/analyzers/tests/test_panic_isolation.rs` | Healthy analyzer ran |
| `HHIER-V2001` | `HeadingHierarchyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No headings found (deep-deep) |
| `HHIER-V2001-DEEP-DEEP-DEEP` | `HeadingHierarchyDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No headings found (deep-deep-deep) |
| `HHIER-V2002` | `HeadingHierarchyDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Empty headings found |
| `HHIER-V2002-DEEP-DEEP` | `HeadingHierarchyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing H1 heading (deep-deep) |
| `HHIER-V2002-DEEP-DEEP-DEEP` | `HeadingHierarchyDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing H1 heading (deep-deep-deep) |
| `HHIER-V2003` | `HeadingHierarchyDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Heading levels skipped |
| `HHIER-V2003-DEEP-DEEP` | `HeadingHierarchyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Multiple H1 headings (deep-deep) |
| `HHIER-V2003-DEEP-DEEP-DEEP` | `HeadingHierarchyDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Multiple H1 headings (deep-deep-deep) |
| `HHIER-V2004` | `HeadingHierarchyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Heading level skipped (deep-deep) |
| `HHIERDEEP001` | `HeadingHierarchyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Heading hierarchy skip |
| `HHIERDEEP002` | `HeadingHierarchyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | First heading is not H1 |
| `HHIERDEEP003` | `HeadingHierarchyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Multiple H1 headings |
| `HORDER001` | `HeadingOrderAnalyzer` | `crates/crawlkit-engine/src/analyzers/heading_analyzers.rs` | Heading level skip detected |
| `HORDER002` | `HeadingOrderAnalyzer` | `crates/crawlkit-engine/src/analyzers/heading_analyzers.rs` | Non-sequential heading order |
| `HOWNAME001` | `HowToMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HowTo missing name |
| `HOWSTEP001` | `HowToMissingStepValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HowTo missing step |
| `HOWTO001` | `HowToSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/how_to_schema.rs` | HowTo schema missing name |
| `HOWTO002` | `HowToSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/how_to_schema.rs` | HowTo schema missing step |
| `HOWTO003` | `HowToSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/how_to_schema.rs` | HowTo step missing name or text |
| `HOWTONAME-V5001` | `HowToNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HowTo missing name |
| `HOWTOSTEP-V5001` | `HowToStepDescriptionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HowTo steps missing description |
| `HOWTOSTEP001` | `HowToStepCountValidator` | `crates/crawlkit-engine/src/analyzers/schema/howto_step_count.rs` | HowTo schema missing step property |
| `HP-V2001` | `HealthPlanSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/health_plan_schema_v2.rs` | HealthPlan schema missing provider |
| `HP001` | `HealthPlanSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/health_plan_schema.rs` | HealthPlan schema missing name |
| `HP002` | `HealthPlanSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/health_plan_schema.rs` | HealthPlan schema missing provider |
| `HPCOV-V6037` | `HealthPlanMissingCoverageAreaValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HealthPlan missing coverageArea |
| `HPID001` | `HealthPlanMissingHealthPlanIdValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HealthPlan missing healthPlanId |
| `HPPRV-V2001` | `HealthPlanMissingProviderValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HealthPlan missing provider |
| `HPPRV-V6036` | `HealthPlanMissingProviderValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | HealthPlan missing provider |
| `HREF-V4001` | `HreflangValidatorV4` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing x-default |
| `HREF-V4003` | `HreflangValidatorV4` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Duplicate hreflang entries |
| `HREF001` | `HreflangValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing x-default hreflang |
| `HREF002` | `HreflangValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Invalid hreflang locale code |
| `HREF003` | `HreflangValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Duplicate hreflang language |
| `HREFFMT-V2001` | `HreflangLocaleFormatDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Invalid hreflang locale format (deep-deep) |
| `HREFFMT-V6095` | `HreflangLocaleFormatDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Hreflang locale format |
| `HREFLOCALE-V5001` | `HreflangLocaleFormatValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Hreflang missing region |
| `HREFMISS-V2001` | `HreflangMissingDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing hreflang tags (deep-deep) |
| `HREFMISS-V6092` | `HreflangMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No hreflang tags |
| `HREFMISS001` | `HreflangMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing hreflang on multilingual site |
| `HREFR002` | `HreflangReciprocalValidator` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Duplicate hreflang language codes |
| `HREFRECIP-V2001` | `HreflangReciprocalDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing reciprocal hreflang (deep-deep-deep) |
| `HREFRECIP-V5001` | `HreflangReciprocalValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing self-referencing hreflang |
| `HREFRECIP-V6093` | `HreflangReciprocalDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Duplicate hreflang languages |
| `HREFRECIP001` | `HreflangReciprocalDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | (no title literal) |
| `HREFSELF-V2001` | `HreflangSelfReferenceValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing hreflang self-reference |
| `HREFSELF-V2002` | `HreflangSelfReferenceValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | (no title literal) |
| `HREFSELF001` | `HreflangSelfReferenceValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing self-referencing hreflang |
| `HREFT001` | `HreflangConsistencyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Hreflang target returns non-200 status |
| `HREFT002` | `HreflangConsistencyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Hreflang target canonical mismatch |
| `HREFT003` | `HreflangConsistencyAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Invalid hreflang locale code |
| `HREFXD-V2001` | `HreflangXDefaultMissingDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing hreflang x-default (deep-deep) |
| `HREFXD-V5001` | `HreflangXDefaultValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing x-default hreflang |
| `HREFXD-V6094` | `HreflangXDefaultMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing hreflang x-default |
| `HREFXD001` | `HreflangXDefaultMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing hreflang x-default |
| `HSTS-V2001` | `StrictTransportSecurityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | Missing Strict-Transport-Security header |
| `HSTS-V2002` | `StrictTransportSecurityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | HSTS max-age is too short |
| `HSTS-V2003` | `StrictTransportSecurityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | HSTS missing includeSubDomains |
| `HSTS-V3001` | `StrictTransportSecurityAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | HSTS missing includeSubDomains |
| `HSTS001` | `HstsPreloadAnalyzer` | `crates/crawlkit-engine/src/analyzers/hsts_analyzer.rs` | HSTS missing includeSubDomains directive |
| `HSTS002` | `HstsPreloadAnalyzer` | `crates/crawlkit-engine/src/analyzers/hsts_analyzer.rs` | HSTS missing preload directive |
| `HSTS003` | `HstsPreloadAnalyzer` | `crates/crawlkit-engine/src/analyzers/hsts_analyzer.rs` | HSTS max-age below recommended minimum |
| `HSTSMAX-V2001` | `HstsMaxAgeDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS max-age below 1 year (deep-deep) |
| `HSTSMAX-V5001` | `HstsMaxAgeValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing HSTS header |
| `HSTSMAX-V5002` | `HstsMaxAgeValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS max-age too low |
| `HSTSMAX-V5003` | `HstsMaxAgeValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS missing max-age |
| `HSTSMAX001` | `HstsMaxAgeDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS max-age below 1 year |
| `HSTSPR-V2001` | `HstsPreloadReadinessAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS missing includeSubDomains |
| `HSTSPR-V2001-DEEP-DEEP` | `HstsPreloadReadyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS preload readiness incomplete (deep-deep) |
| `HSTSPR-V2002` | `HstsPreloadReadinessAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS missing preload |
| `HSTSPR-V2003` | `HstsPreloadReadinessAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS max-age below preload minimum |
| `HSTSPR001` | `HstsPreloadReadinessAnalyzer` | `crates/crawlkit-engine/src/analyzers/mixed_content_analyzers.rs` | HSTS missing includeSubDomains for preload |
| `HSTSPR001-DEEP` | `HstsPreloadReadyDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS preload readiness incomplete |
| `HSTSPR002` | `HstsPreloadReadinessAnalyzer` | `crates/crawlkit-engine/src/analyzers/mixed_content_analyzers.rs` | HSTS missing preload directive |
| `HSTSPR003` | `HstsPreloadReadinessAnalyzer` | `crates/crawlkit-engine/src/analyzers/mixed_content_analyzers.rs` | HSTS max-age too low for preload |
| `HSTSPRE-V5001` | `HstsPreloadValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS missing preload |
| `HSTSPRELIST-V6064` | `HstsPreloadListCheckValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS preload readiness incomplete |
| `HSTSPRELOAD001` | `HstsPreloadListValidator` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | HSTS preload directive present without meeting preload requirements |
| `HSTSSUB-V5001` | `HstsIncludeSubDomainsValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS missing includeSubDomains |
| `HSTSTHRESH-V6063` | `HstsMaxAgeThresholdValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | HSTS max-age below recommended threshold |
| `HTTP001` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Missing status code |
| `HTTP002` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Slow response time |
| `HTTP003` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Possible soft 404 — empty body |
| `HTTP004` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Page not found (404) |
| `HTTP005` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | (no title literal) |
| `HTTP006` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | (no title literal) |
| `HTTP007` | `HttpStatusAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Possible soft 404 — error page content detected |
| `HTTPVER001` | `HttpVersionAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | HTTP/1.0 response when HTTP/2 may be available |
| `HTTPVER002` | `HttpVersionAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | HTTP/1.1 response |
| `IMG-V2001` | `ImageAccessibilityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Images missing alt attribute |
| `IMG001` | `ImageAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Image missing alt text |
| `IMG003` | `ImageAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Lazy-loaded images |
| `IMG004` | `ImageAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Images missing dimensions |
| `IMG005` | `ImageAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Non-modern image formats detected |
| `IMGACC001` | `ImageAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/image_accessibility_analyzers.rs` | Image missing alt attribute |
| `IMGACC002` | `ImageAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/image_accessibility_analyzers.rs` | Image with empty alt on non-decorative image |
| `IMGACC003` | `ImageAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/image_accessibility_analyzers.rs` | Image alt text identical to filename |
| `IMGALT-V2001` | `ImageAltTextDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Images missing alt |
| `IMGALT-V2001-DEEP-DEEP` | `ImageAltTextDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Images missing alt attribute (deep-deep) |
| `IMGALT-V2001-DEEP-DEEP-DEEP` | `ImageAltTextDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Images missing alt attribute (deep-deep-deep) |
| `IMGALT-V2002-DEEP-DEEP` | `ImageAltTextDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Images with empty alt text (deep-deep) |
| `IMGALT-V2003` | `ImageAltTextDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Generic alt text |
| `IMGALT001` | `ImageAltTextAnalyzer` | `crates/crawlkit-engine/src/analyzers/basic_accessibility_analyzers.rs` | Image missing alt text |
| `IMGALTDEEP001` | `ImageAltTextDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Images missing alt text |
| `IMGALTDEEP002` | `ImageAltTextDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Alt text too long |
| `IMGALTEMPTY-V6117` | `ImageAltEmptyDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Empty alt attributes |
| `IMGALTMISS-V6116` | `ImageAltMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Images missing alt text |
| `IMGAR001` | `ImageAspectRatioValidator` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Unusual image aspect ratio |
| `IMGDECPAT-V6118` | `ImageAltDecorativePatternValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Suspiciously generic alt text |
| `IMGDIM-V5001` | `ImageDimensionsValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/performance.rs` | Images missing dimensions |
| `IMGFMT-V5001` | `ImageModernFormatValidator` | `crates/crawlkit-engine/src/analyzers/v2/performance.rs` | No modern image formats |
| `IMGFS001` | `ImageFileSizeValidator` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Inline data URI image |
| `IMGLAZY-V5001` | `ImageLazyLoadingValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/performance.rs` | No lazy loading on images |
| `INTDEPTH-V2001` | `InternalLinksDepthDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Many deeply nested internal links (deep-deep) |
| `INTDEPTH-V6104` | `InternalLinksDepthDistributionValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Many deep internal links |
| `INTDEPTH001` | `InternalLinksDepthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Many deeply nested internal links |
| `INTDEPTH002` | `InternalLinksDepthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Very deep internal link targets |
| `INTDIV-V2001` | `InternalLinksDiversityDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Low internal link diversity (deep-deep) |
| `INTDIV-V6103` | `InternalLinksDiversityValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Low internal link text diversity |
| `INTDIV001` | `InternalLinksDiversityDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Low internal link diversity |
| `INTLINKQ-V2001` | `InternalLinkQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Self-referencing links |
| `INTLINKQ-V2001-DEEP` | `InternalLinkQualityDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | High nofollow internal link ratio (deep) |
| `INTLINKQ-V2002` | `InternalLinkQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | All internal links nofollowed |
| `INTLINKQ-V2002-DEEP` | `InternalLinkQualityDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Internal links without anchor text (deep) |
| `INTLINKQ001` | `InternalLinkQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Page contains self-links |
| `INTLINKQ002` | `InternalLinkQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | All internal links are nofollowed |
| `INTLINKQ003` | `InternalLinkQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Internal links with empty anchor text |
| `INTOPIC001` | `InternalLinkTopicalAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Internal link anchor text lacks topical relevance |
| `INV-V2001` | `InvoiceSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/invoice_schema_v2.rs` | Invoice schema missing account |
| `INV001` | `InvoiceSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/invoice_schema.rs` | Invoice schema missing accountId |
| `INV002` | `InvoiceSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/invoice_schema.rs` | Invoice schema missing dueDate |
| `INVACCT-V2001` | `InvoiceMissingAccountValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Invoice missing account |
| `INVACCT-V6038` | `InvoiceMissingAccountValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Invoice missing account |
| `INVPAY-V6039` | `InvoiceMissingPaymentDueDateValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Invoice missing paymentDueDate |
| `INVSTATUS001` | `InvoiceMissingPaymentStatusValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Invoice missing paymentStatus |
| `ISEO001` | `InternationalSeoAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Hreflang URL locale mismatch |
| `ISEO002` | `InternationalSeoAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing x-default in enhanced hreflang |
| `ISEO003` | `InternationalSeoAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Duplicate hreflang language |
| `ISEO004` | `InternationalSeoAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Single-language page without hreflang |
| `ISEO005` | `InternationalSeoAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Canonical chain detected |
| `ISEO007` | `AdvancedCanonicalAnalyzer` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Hreflang may point to non-canonical URL |
| `ITEMLIST001` | `ItemListSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/item_list_schema.rs` | ItemList schema missing itemListElement |
| `ITEMLIST002` | `ItemListSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/item_list_schema.rs` | ItemList schema itemListElement is empty |
| `JOB001` | `JobPostingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/job_posting_schema.rs` | JobPosting schema missing title |
| `JOB002` | `JobPostingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/job_posting_schema.rs` | JobPosting schema missing datePosted |
| `JOB003` | `JobPostingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/job_posting_schema.rs` | JobPosting schema missing validThrough |
| `JOBCURR-V5001` | `JobPostingSalaryCurrencyValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Invalid salary currency |
| `JOBEMP-V5001` | `JobPostingEmploymentTypeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Invalid employmentType |
| `JOBTITLE-V2001` | `JobPostingMissingTitleValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | JobPosting missing title |
| `JOBVALID001` | `JobPostingValidThroughValidator` | `crates/crawlkit-engine/src/analyzers/schema/job_posting_valid_through.rs` | JobPosting missing validThrough |
| `JOBVT-V5001` | `JobPostingValidThroughValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | JobPosting missing validThrough |
| `JSAL001` | `JobPostingSalaryValidator` | `crates/crawlkit-engine/src/analyzers/schema/job_posting_salary.rs` | JobPosting missing baseSalary |
| `JSAL002` | `JobPostingSalaryValidator` | `crates/crawlkit-engine/src/analyzers/schema/job_posting_salary.rs` | JobPosting missing employmentType |
| `JSERR001` | `JsErrorAnalyzer` | `crates/crawlkit-engine/src/analyzers/js_error_analyzers.rs` | Uncaught JavaScript error during rendering |
| `JSERR002` | `JsErrorAnalyzer` | `crates/crawlkit-engine/src/analyzers/js_error_analyzers.rs` | Elevated JavaScript console errors |
| `KW001` | `KeywordAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Top TF-IDF keywords |
| `KW002` | `KeywordAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Keyword density |
| `KW003` | `KeywordAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Prominent keywords detected |
| `KW004` | `KeywordAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Keyword co-occurrence |
| `LAND001` | `LandmarkRegionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_analyzers.rs` | Missing main landmark region |
| `LAND002` | `LandmarkRegionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_analyzers.rs` | Missing navigation landmark |
| `LAND003` | `LandmarkRegionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_analyzers.rs` | Missing banner/header landmark |
| `LANDBAN-V2001` | `LandmarkBannerDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing banner landmark (deep) |
| `LANDBAN001` | `LandmarkBannerAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_heading_validator_analyzers.rs` | Page missing banner/header landmark |
| `LANDBANNER-V5001` | `LandmarkBannerValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing banner landmark |
| `LANDCINFO-V2001` | `LandmarkContentinfoDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing contentinfo landmark (deep) |
| `LANDCINFO-V6124` | `LandmarkContentinfoValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing contentinfo landmark |
| `LANDCOMP-V2001` | `LandmarkComplementaryDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No complementary landmark (deep) |
| `LANDCOMP-V6125` | `LandmarkComplementaryValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No complementary landmark |
| `LANDFORM001` | `LandformSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/landform_schema.rs` | Landform schema missing name |
| `LANDFORM002` | `LandformSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/landform_schema.rs` | Landform schema missing geographic data |
| `LANDMAIN-V2001` | `LandmarkMainDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing main landmark (deep) |
| `LANDMAIN-V5001` | `LandmarkMainValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing main landmark |
| `LANDMAIN-V5002` | `LandmarkMainValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Multiple main landmarks |
| `LANDMAIN001` | `LandmarkMainAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_heading_validator_analyzers.rs` | Page missing main landmark |
| `LANDMARK001` | `LandmarksOrHistoricalBuildingsSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/landmarks_or_historical_buildings_schema.rs` | LandmarksOrHistoricalBuildings schema missing name |
| `LANDMARK002` | `LandmarksOrHistoricalBuildingsSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/landmarks_or_historical_buildings_schema.rs` | LandmarksOrHistoricalBuildings schema missing location |
| `LANDNAV-V2001` | `LandmarkNavDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing navigation landmark (deep) |
| `LANDNAV-V5001` | `LandmarkNavValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing navigation landmark |
| `LANDNAV001` | `LandmarkNavAnalyzer` | `crates/crawlkit-engine/src/analyzers/landmark_heading_validator_analyzers.rs` | Page missing navigation landmark |
| `LANG-V2001` | `LanguageAttributeAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Missing lang attribute |
| `LANG001` | `LanguageAttributeAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing html lang attribute |
| `LANG002` | `LanguageAttributeAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Language attribute mismatch |
| `LANG003` | `LanguageAttributeAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Empty lang attribute |
| `LANGACC001` | `LanguageAttributeAnalyzer` | `crates/crawlkit-engine/src/analyzers/language_accessibility_analyzers.rs` | Missing html lang attribute |
| `LANGACC002` | `LanguageAttributeAnalyzer` | `crates/crawlkit-engine/src/analyzers/language_accessibility_analyzers.rs` | Lang attribute value too short |
| `LANGATTR-V2001` | `LanguageAttributesDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing html lang |
| `LANGATTR-V2001-DEEP-DEEP` | `LanguageAttributesDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Missing lang attribute (deep-deep) |
| `LANGATTRDEEP001` | `LanguageAttributesDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Missing html lang attribute |
| `LANGATTRDEEP002` | `LanguageAttributesDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Language code may be too specific |
| `LANGATTRDEEP003` | `LanguageAttributesDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Language mismatch between HTML and meta |
| `LAZYIMG001` | `ImageLazyLoadAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Images without lazy loading or dimensions |
| `LAZYIMG002` | `ImageLazyLoadAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Above-the-fold images with lazy loading |
| `LBAMEN-V6008` | `LodgingBusinessMissingAmenityFeatureValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | LodgingBusiness missing amenityFeature |
| `LBGEO-V6060` | `LocalBusinessMissingGeoValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | LocalBusiness missing geo |
| `LBH001` | `LocalBusinessHoursValidator` | `crates/crawlkit-engine/src/analyzers/schema/local_business_hours.rs` | LocalBusiness missing openingHours |
| `LBH002` | `LocalBusinessHoursValidator` | `crates/crawlkit-engine/src/analyzers/schema/local_business_hours.rs` | LocalBusiness openingHours in invalid format |
| `LBIZ001` | `LocalBusinessSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/local_business_schema.rs` | LocalBusiness schema missing name |
| `LBIZ002` | `LocalBusinessSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/local_business_schema.rs` | LocalBusiness schema missing address |
| `LBPI-V2001` | `LocalBusinessMissingNpiValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | LocalBusiness missing identifier (NPI) |
| `LBSTAR-V6007` | `LodgingBusinessMissingStarRatingValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | LodgingBusiness missing starRating |
| `LFNAME-V6011` | `LandformMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Landform missing name |
| `LINK001` | `LinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Link counts |
| `LINK002` | `LinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Link on broken page |
| `LINK003` | `LinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Nofollow links present |
| `LINK004` | `LinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Empty anchor text |
| `LINK005` | `LinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Very short anchor text |
| `LINK006` | `LinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Orphan page |
| `LINKDUP-V6115` | `LinkTextDuplicateValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Duplicate link text |
| `LINKEEMPTY-V6114` | `LinkTextEmptyDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Empty link text |
| `LINKGEN-V6113` | `LinkTextGenericDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Generic link text |
| `LINKSC001` | `LinkQualityScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | No links on page |
| `LINKSC002` | `LinkQualityScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | High nofollow link ratio |
| `LINKSC003` | `LinkQualityScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Links without anchor text |
| `LINKTEXT001` | `LinkTextAnalyzer` | `crates/crawlkit-engine/src/analyzers/basic_accessibility_analyzers.rs` | Link with empty text |
| `LINKTEXT002` | `LinkTextAnalyzer` | `crates/crawlkit-engine/src/analyzers/basic_accessibility_analyzers.rs` | Link with generic text |
| `LINKTQ-V2001` | `LinkTextQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Generic link text |
| `LINKTQ-V2001-DEEP` | `LinkTextQualityDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Links without text (deep) |
| `LINKTQ-V2001-DEEP-DEEP` | `LinkTextQualityDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Links without text (deep-deep) |
| `LINKTQ-V2002-DEEP` | `LinkTextQualityDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Generic link text (deep) |
| `LINKTQ-V2002-V2` | `LinkTextQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Empty link text |
| `LINKTQ001` | `LinkTextQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Generic link text detected |
| `LINKTQ002` | `LinkTextQualityAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Links without accessible text |
| `LMNAME-V6012` | `LandmarkMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Landmark missing name |
| `LNKACC001` | `LinkAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/link_accessibility_analyzers.rs` | Link with empty text content |
| `LNKACC002` | `LinkAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/link_accessibility_analyzers.rs` | Link with generic text |
| `LNKACC003` | `LinkAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/link_accessibility_analyzers.rs` | Link with non-descriptive text |
| `LODGE001` | `LodgingBusinessSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/lodging_business_schema.rs` | LodgingBusiness schema missing name |
| `LODGE002` | `LodgingBusinessSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/lodging_business_schema.rs` | LodgingBusiness schema missing address |
| `LODGE003` | `LodgingBusinessSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/lodging_business_schema.rs` | LodgingBusiness schema missing telephone |
| `MD001` | `MicrodataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Microdata itemscope without itemprop |
| `MD002` | `MicrodataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Microdata missing required properties |
| `MD003` | `MicrodataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Microdata @type not in Schema.org vocabulary |
| `MDESC-PX001` | `MetaDescriptionPixelWidthAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description exceeds SERP pixel width |
| `MDESC-PX002` | `MetaDescriptionPixelWidthAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description too short for SERP display |
| `META-V3001` | `MetaDescriptionLengthAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing meta description |
| `META-V3002` | `MetaDescriptionLengthAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description very short |
| `META-V3003` | `MetaDescriptionLengthAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description may truncate |
| `META001` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing page title |
| `META002` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title too short |
| `META003` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title too long |
| `META004` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing meta description |
| `META005` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description too short |
| `META006` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description too long |
| `META009` | `MetaTagAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing viewport meta tag |
| `METADEEP-V2001` | `MetaDescriptionDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description too short |
| `METADEEP-V2002` | `MetaDescriptionDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description too long |
| `METADEEP001` | `MetaDescriptionDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description too short |
| `METADEEP002` | `MetaDescriptionDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description too long |
| `METADEEP003` | `MetaDescriptionDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description contains quotes |
| `METADEEP004` | `MetaDescriptionDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Meta description has repetitive words |
| `METADESCMISS-V2001` | `MetaDescriptionMissingDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing meta description (deep-deep) |
| `METADESCMISS-V6085` | `MetaDescriptionMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing meta description |
| `METADESCMISS001` | `MetaDescriptionMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing meta description |
| `METADESCUNIQ-V2001` | `MetaDescriptionUniquenessDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description identical to title (deep-deep) |
| `METADESCUNIQ-V6087` | `MetaDescriptionUniquenessValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description matches title |
| `METADESCUNIQ001` | `MetaDescriptionUniquenessDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description identical to title |
| `METADESSSHORT-V2001` | `MetaDescriptionTooShortDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description too short (deep-deep) |
| `METADESSSHORT-V6086` | `MetaDescriptionTooShortValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description too short |
| `METADESSSHORT001` | `MetaDescriptionTooShortDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description too short |
| `METAKEY-V5001` | `MetaDescriptionKeywordValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description missing title keywords |
| `METALEN-V2001` | `MetaDescriptionLengthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description too long (deep) |
| `METALEN-V2002` | `MetaDescriptionLengthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Meta description too short (deep) |
| `METAQLT-V2001` | `MetaDescriptionQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description too short |
| `METAQLT-V2002` | `MetaDescriptionQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description may truncate |
| `METAUNIQ-V5001` | `MetaDescriptionUniqueValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Description identical to title |
| `MIXCONT-V2001` | `MixedContentDetectionAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `MIXCONT-V2005` | `MixedContentDetectionAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | No upgrade-insecure-requests CSP |
| `MIXCONT001` | `MixedContentDetectionAnalyzer` | `crates/crawlkit-engine/src/analyzers/mixed_content_analyzers.rs` | Mixed content: active script |
| `MIXCONT002` | `MixedContentDetectionAnalyzer` | `crates/crawlkit-engine/src/analyzers/mixed_content_analyzers.rs` | Mixed content: passive image |
| `MIXCONT003` | `MixedContentDetectionAnalyzer` | `crates/crawlkit-engine/src/analyzers/mixed_content_analyzers.rs` | Mixed content: stylesheet/resource |
| `MIXCSS-V5001` | `MixedContentStylesheetValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `MIXED001` | `MixedContentAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | HTTP resources on HTTPS page |
| `MIXED002` | `MixedContentAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Form submissions over HTTP |
| `MIXFRM001` | `MixedContentFormValidator` | `crates/crawlkit-engine/src/analyzers/mixed_content_validator_analyzers.rs` | HTTP form action on HTTPS page |
| `MIXIFRAME-V2001` | `MixedContentIframeDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Mixed content in iframes (deep-deep) |
| `MIXIFRAME-V6068` | `MixedContentIframeValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Mixed content iframe detected |
| `MIXIFRAME001` | `MixedContentIframeDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Mixed content in iframes |
| `MIXIMG001` | `MixedContentImageValidator` | `crates/crawlkit-engine/src/analyzers/mixed_content_validator_analyzers.rs` | HTTP image sources on HTTPS page |
| `MIXPROT-V2002` | `MixedProtocolRedirectValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | HTTPS redirecting to HTTP |
| `MIXSCR001` | `MixedContentScriptValidator` | `crates/crawlkit-engine/src/analyzers/mixed_content_validator_analyzers.rs` | HTTP script sources on HTTPS page |
| `MIXSCRIPT-V5001` | `MixedContentScriptValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `MOB001` | `MobileFriendlinessChecker` | `crates/crawlkit-engine/src/analyzers/mobile_analyzers.rs` | Missing viewport meta tag |
| `MOB002` | `MobileFriendlinessChecker` | `crates/crawlkit-engine/src/analyzers/mobile_analyzers.rs` | Viewport missing width directive |
| `MOB003` | `MobileFriendlinessChecker` | `crates/crawlkit-engine/src/analyzers/mobile_analyzers.rs` | Viewport width is not device-width |
| `MOB004` | `MobileFriendlinessChecker` | `crates/crawlkit-engine/src/analyzers/mobile_analyzers.rs` | Zooming is disabled (user-scalable=no) |
| `MOB005` | `MobileFriendlinessChecker` | `crates/crawlkit-engine/src/analyzers/mobile_analyzers.rs` | Maximum scale restricted |
| `MOB009` | `MobileFriendlinessChecker` | `crates/crawlkit-engine/src/analyzers/mobile_analyzers.rs` | Non-standard initial scale |
| `MOBVIEW001` | `MobileViewportAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Viewport missing initial-scale |
| `MOBVIEW002` | `MobileViewportAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Viewport width not set to device-width |
| `MOBVIEW003` | `MobileViewportAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Viewport disables user scaling |
| `MOVACT-V6056` | `MovieMissingActorValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Movie missing actor |
| `MOVDATE-V6029` | `MovieMissingDateCreatedValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Movie missing dateCreated |
| `MOVDIR-V2001` | `MovieMissingDirectorValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Movie missing director |
| `MOVDIR-V6027` | `MovieMissingDirectorValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Movie missing director |
| `MOVDUR-V6028` | `MovieMissingDurationValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Movie missing duration |
| `MOVIE-V2001` | `MovieSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/movie_schema_v2.rs` | Movie schema missing director |
| `MOVIE001` | `MovieSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/movie_schema.rs` | Movie schema missing name |
| `MOVIE002` | `MovieSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/movie_schema.rs` | Movie schema missing director |
| `MOVIE003` | `MovieSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/movie_schema.rs` | Movie schema missing dateCreated |
| `MOVRATING001` | `MovieMissingAggregateRatingValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Movie missing aggregateRating |
| `MUSALB-V2001` | `MusicAlbumSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/music_album_schema_v2.rs` | MusicAlbum schema missing byArtist |
| `MUSALB-V6033` | `MusicRecordingMissingAlbumValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | MusicRecording missing inAlbum |
| `MUSALB001` | `MusicAlbumSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/music_album_schema.rs` | MusicAlbum schema missing name |
| `MUSALB002` | `MusicAlbumSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/music_album_schema.rs` | MusicAlbum schema missing byArtist |
| `MUSART-V6032` | `MusicRecordingMissingByArtistValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | MusicRecording missing byArtist |
| `MUSCTRACKS-V2001` | `MusicAlbumMissingTracksValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | MusicAlbum missing tracks |
| `MUSREC001` | `MusicRecordingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/music_recording_schema.rs` | MusicRecording schema missing name |
| `MUSTRACKS001` | `MusicAlbumMissingNumberOfTracksValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | MusicAlbum missing numberOfTracks |
| `NAP001` | `LocalBusinessNapAnalyzer` | `crates/crawlkit-engine/src/analyzers/schema/local_business_nap.rs` | LocalBusiness schema missing telephone |
| `NAP002` | `LocalBusinessNapAnalyzer` | `crates/crawlkit-engine/src/analyzers/schema/local_business_nap.rs` | LocalBusiness schema missing openingHours |
| `NGO001` | `NGOSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/ngo_schema.rs` | NGO schema missing name |
| `NGO002` | `NGOSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/ngo_schema.rs` | NGO schema missing address |
| `NGO003` | `NGOSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/ngo_schema.rs` | NGO schema missing url |
| `NGONAME-V6018` | `NGOMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | NGO missing name |
| `NOFOLLOW-V2001` | `InternalNofollowOveruseValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Extreme nofollow overuse |
| `NOFOLLOW-V2002` | `InternalNofollowOveruseValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | High nofollow ratio |
| `NPI001` | `LocalBusinessNpiValidator` | `crates/crawlkit-engine/src/analyzers/schema/local_business_npi.rs` | Medical LocalBusiness missing NPI |
| `OCCUP-V2001` | `OccupationSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/occupation_schema_v2.rs` | Occupation schema missing occupationalCategory |
| `OCCUP001` | `OccupationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/occupation_schema.rs` | Occupation schema missing name |
| `OCCUP002` | `OccupationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/occupation_schema.rs` | Occupation schema missing occupationalCategory |
| `OFFER001` | `OfferSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/offer_schema.rs` | Offer schema missing price |
| `OFFER002` | `OfferSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/offer_schema.rs` | Offer schema missing priceCurrency |
| `OFFER003` | `OfferSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/offer_schema.rs` | Offer schema missing availability |
| `OGAUDIO001` | `OpenGraphAudioAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | og:audio present but missing og:audio:url |
| `OGAUDIO002` | `OpenGraphAudioAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | og:audio present but missing og:audio:type |
| `OGIMG001` | `OpenGraphImageValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Invalid OG image URL |
| `OGIMG002` | `OpenGraphImageValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG image missing dimensions |
| `OGIMG003` | `OpenGraphImageValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG image format not supported |
| `OGSITE001` | `OpenGraphSiteNameValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Empty og:site_name |
| `OGSITE002` | `OpenGraphSiteNameValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Missing og:site_name |
| `OGURL001` | `OpenGraphUrlValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | og:url doesn't match page URL |
| `OGVID001` | `OpenGraphVideoAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | og:video present but missing og:video:url |
| `OGVID002` | `OpenGraphVideoAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | og:video present but missing og:video:type |
| `OLOGO001` | `OrganizationLogoValidator` | `crates/crawlkit-engine/src/analyzers/schema/organization_logo.rs` | Organization missing logo |
| `OLOGO002` | `OrganizationLogoValidator` | `crates/crawlkit-engine/src/analyzers/schema/organization_logo.rs` | Organization logo URL invalid format |
| `OPDESC-V2001` | `OpenSearchDescriptionValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No OpenSearch description |
| `OPDESC001` | `OpenSearchDescriptionValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing OpenSearch description |
| `OPSEARCH001` | `OpenSearchValidator` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | No OpenSearch description link found |
| `ORG001` | `OrganizationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/organization_schema.rs` | Organization schema missing name |
| `ORG002` | `OrganizationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/organization_schema.rs` | Organization schema missing url |
| `ORG003` | `OrganizationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/organization_schema.rs` | Organization schema missing logo |
| `ORGCONT-V5001` | `OrganizationContactValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Organization missing contactPoint |
| `ORGLOGO-V5001` | `OrganizationLogoUrlValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Organization logo missing URL |
| `ORGNAME-V2001` | `OrganizationMissingNameValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Organization missing name |
| `ORGURL-V5001` | `OrganizationUrlValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Organization missing URL |
| `ORSAMEAS001` | `OrganizationSameAsValidator` | `crates/crawlkit-engine/src/analyzers/schema/organization_sameas.rs` | Organization sameAs URL is invalid |
| `PAG001` | `PaginationAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Missing rel=\ |
| `PAG002` | `PaginationAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Infinite scroll pagination detected |
| `PAG004` | `PaginationAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Excessive pagination depth |
| `PAGDEP-V2001` | `PaginationDepthAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Paginated URL detected |
| `PASNAME-V6019` | `PerformingArtsSeriesMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | PerformingArtsSeries missing name |
| `PERFORMARTS001` | `PerformingArtsSeriesSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/performing_arts_series_schema.rs` | PerformingArtsSeries schema missing name |
| `PERFORMARTS002` | `PerformingArtsSeriesSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/performing_arts_series_schema.rs` | PerformingArtsSeries schema missing organizer/performer |
| `PERFORMARTS003` | `PerformingArtsSeriesSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/performing_arts_series_schema.rs` | PerformingArtsSeries schema missing events |
| `PERM-V2001` | `PermissionsPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/permissions_policy_v2_analyzers.rs` | Permissions-Policy header missing |
| `PERM-V2001-SCHEMA` | `PermitSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/permit_schema_v2.rs` | Permit schema missing permitNumber |
| `PERM-V2002` | `PermissionsPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/permissions_policy_v2_analyzers.rs` | (no title literal) |
| `PERM-V3001` | `PermissionsPolicyAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | Permissions-Policy missing camera restriction |
| `PERM001` | `PermissionPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Missing Permissions-Policy header |
| `PERM002` | `PermissionPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | (no title literal) |
| `PERMFROM001` | `PermitMissingValidFromValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Permit missing validFrom |
| `PERMISS-V6041` | `PermitMissingIssuedByValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Permit missing issuedBy |
| `PERMIT001` | `PermitSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/permit_schema.rs` | Permit schema missing permitNumber |
| `PERMIT002` | `PermitSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/permit_schema.rs` | Permit schema missing issuedBy |
| `PERMNUM-V2001` | `PermitMissingPermitNumberValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Permit missing permitNumber |
| `PERMNUM-V6040` | `PermitMissingPermitNumberValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Permit missing permitNumber |
| `PERMP-V2001` | `PermissionsPolicyDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Permissions-Policy |
| `PERMP-V2001-DEEP-DEEP` | `PermissionsPolicyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Permissions-Policy header (deep-deep) |
| `PERMPDEEP001` | `PermissionsPolicyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Missing Permissions-Policy header |
| `PERMPDEEP002` | `PermissionsPolicyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | (no title literal) |
| `PERS001` | `PersonSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/person_schema.rs` | Person schema missing name |
| `PERS002` | `PersonSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/person_schema.rs` | Person schema missing sameAs |
| `PERSNAME-V2001` | `PersonMissingNameValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Person missing name |
| `PERSURL-V5001` | `PersonUrlValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Person missing url |
| `PJOB001` | `PersonJobTitleValidator` | `crates/crawlkit-engine/src/analyzers/schema/person_job_title.rs` | Person missing jobTitle |
| `PJOB002` | `PersonJobTitleValidator` | `crates/crawlkit-engine/src/analyzers/schema/person_job_title.rs` | Person missing worksFor |
| `PLAN-V2001` | `PlanSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/plan_schema_v2.rs` | Plan schema missing description |
| `PLAN001` | `PlanSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/plan_schema.rs` | Plan schema missing name |
| `PLAN002` | `PlanSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/plan_schema.rs` | Plan schema missing description |
| `PLANABOUT-V6043` | `PlanMissingAboutValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Plan missing about |
| `PLANBEN001` | `PlanMissingBenefitsValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Plan missing benefits |
| `PLANDESC-V2001` | `PlanMissingDescriptionValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Plan missing description |
| `PLANDESC-V6042` | `PlanMissingDescriptionValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Plan missing description |
| `PLAYBOOK-V2001` | `PlaybookSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/playbook_schema_v2.rs` | Playbook schema missing name |
| `PLAYBOOK001` | `PlaybookSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/playbook_schema.rs` | Playbook schema missing name |
| `PLAYBOOK002` | `PlaybookSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/playbook_schema.rs` | Playbook schema missing step |
| `PLAYLIST001` | `PlaylistSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/playlist_schema.rs` | Playlist schema missing name |
| `PLAYLIST002` | `PlaylistSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/playlist_schema.rs` | Playlist schema has zero tracks |
| `PLAYLIST003` | `PlaylistSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/playlist_schema.rs` | Playlist schema missing track information |
| `PLNUM-V6004` | `PlaylistMissingNumberOfItemsValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Playlist missing numberOfItems |
| `PMODEL-V2001` | `ProductModelSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/product_model_schema_v2.rs` | ProductModel schema missing brand |
| `PMODEL001` | `ProductModelSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_model_schema.rs` | ProductModel schema missing name |
| `PMODEL002` | `ProductModelSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_model_schema.rs` | ProductModel schema missing brand |
| `PPAYDP001` | `PermissionsPolicyPaymentDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Payment API access not explicitly denied |
| `PPCAM-V5001` | `PermissionsPolicyCameraValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Camera access not explicitly denied |
| `PPERM001` | `PermissionsPolicyAnalyzerNew` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Permissions-Policy header missing |
| `PPERM002` | `PermissionsPolicyAnalyzerNew` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Permissions-Policy allows camera by default |
| `PPFULL-V6074` | `PermissionsPolicyFullscreenValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Fullscreen access not restricted to self |
| `PPFULLDP001` | `PermissionsPolicyFullscreenDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Fullscreen API access not explicitly restricted |
| `PPGEO-V5001` | `PermissionsPolicyGeolocationValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Geolocation not explicitly denied |
| `PPMICRO-V5001` | `PermissionsPolicyMicrophoneValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Microphone access not explicitly denied |
| `PPPAY-V6073` | `PermissionsPolicyPaymentValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Payment API access not explicitly denied |
| `PPXR-V6075` | `PermissionsPolicyXrVrValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | XR spatial tracking access not denied |
| `PPXRDP001` | `PermissionsPolicyXrVrDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | (no title literal) |
| `PRECON-V5001` | `PreconnectHintValidator` | `crates/crawlkit-engine/src/analyzers/v2/performance.rs` | No preconnect hints |
| `PRELOAD001` | `PreloadHintAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Critical resources missing preload hints |
| `PRELOAD002` | `PreloadHintAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Too many preload hints |
| `PREV001` | `ProductReviewValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_review.rs` | Product review missing reviewRating |
| `PREV002` | `ProductReviewValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_review.rs` | Product review missing author |
| `PRICE001` | `PricingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Price present but priceCurrency missing |
| `PRICE002` | `PricingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | priceValidUntil missing from offer |
| `PRODAVAIL001` | `ProductAvailabilityValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_availability.rs` | Product schema missing offers |
| `PRODBRAND-V6021` | `ProductMissingBrandValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product missing brand |
| `PRODCAT-V6022` | `ProductMissingCategoryValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product missing category |
| `PRODIMG-V5001` | `ProductImageValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product has empty image array |
| `PRODIMG-V5002` | `ProductImageValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product missing image |
| `PRODPRICE-V5001` | `ProductPriceValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product offer missing price/currency |
| `PRODREV-V6023` | `ProductMissingReviewValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product missing review/aggregateRating |
| `PRODSKU-V6054` | `ProductMissingSkuValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Product missing sku |
| `PROFFER001` | `ProductOfferValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_offer.rs` | Product missing offers |
| `PROFFER002` | `ProductOfferValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_offer.rs` | Product offer missing priceCurrency |
| `PROFFER003` | `ProductOfferValidator` | `crates/crawlkit-engine/src/analyzers/schema/product_offer.rs` | Invalid priceCurrency value |
| `PVAR001` | `ProductVariantAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Product schema missing variant information |
| `PVAR002` | `ProductVariantAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Product schema has offers but no availability |
| `QUEST001` | `QuestSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/quest_schema.rs` | Quest schema missing name |
| `QUEST002` | `QuestSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/quest_schema.rs` | Quest schema missing questType |
| `RDFA001` | `RdfaValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | RDFa attributes present but missing vocab |
| `RDFA002` | `RdfaValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | RDFa attributes present but missing typeof |
| `RDFA003` | `RdfaValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | RDFa uses deprecated vocabulary |
| `READ001` | `EnhancedReadabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Flesch-Kincaid Grade Level |
| `READ002` | `EnhancedReadabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Coleman-Liau Index |
| `READ003` | `EnhancedReadabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Automated Readability Index |
| `READ004` | `EnhancedReadabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Gunning Fog Index |
| `READ005` | `EnhancedReadabilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Flesch Reading Ease score |
| `RECCOOK001` | `RecipeMissingCookTimeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Recipe missing cookTime |
| `RECIPE001` | `RecipeSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_schema.rs` | Recipe schema missing name |
| `RECIPE002` | `RecipeSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_schema.rs` | Recipe schema missing cookTime |
| `RECIPE003` | `RecipeSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_schema.rs` | Recipe schema missing recipeIngredient |
| `RECIPECOOK001` | `RecipeCookTimeValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_cook_time.rs` | Recipe missing cookTime |
| `RECIPECOOK002` | `RecipeCookTimeValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_cook_time.rs` | Recipe missing prepTime |
| `RECIPECOOK003` | `RecipeCookTimeValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_cook_time.rs` | Recipe missing totalTime |
| `RECIPEING-V5001` | `RecipeIngredientsValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Recipe has empty ingredients |
| `RECIPEING-V5002` | `RecipeIngredientsValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Recipe missing ingredients |
| `RECIPENAME-V2001` | `RecipeMissingNameValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Recipe missing name |
| `RECIPEPT-V5001` | `RecipePrepTimeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Recipe missing prepTime |
| `REDIR001` | `RedirectChainAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Long redirect chain |
| `REDIR002` | `RedirectChainAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Redirect loop detected |
| `REDIR003` | `RedirectChainAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Mixed-protocol redirect |
| `REDIR004` | `RedirectChainAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Single redirect detected |
| `REF-V2001` | `ReferrerPolicyAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | Missing Referrer-Policy header |
| `REF001` | `ReferrerPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Missing Referrer-Policy header |
| `REF002` | `ReferrerPolicyAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Referrer-Policy set to unsafe-url |
| `RES001` | `ResourceCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Excessive total resource count |
| `RES002` | `ResourceCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Excessive JavaScript file count |
| `RES003` | `ResourceCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Excessive image count |
| `RESSIZE001` | `ResourceSizeAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Single resource exceeds 500KB |
| `RESSIZE002` | `ResourceSizeAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Estimated total page size exceeds 5MB |
| `REV001` | `ReviewSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/review_schema.rs` | AggregateRating missing reviewCount |
| `REV002` | `ReviewSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/review_schema.rs` | AggregateRating ratingValue out of range |
| `REV003` | `ReviewSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/review_schema.rs` | Review schema missing author |
| `RNUT001` | `RecipeNutritionValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_nutrition.rs` | Recipe missing nutrition information |
| `RNUT002` | `RecipeNutritionValidator` | `crates/crawlkit-engine/src/analyzers/schema/recipe_nutrition.rs` | Recipe nutrition missing calories |
| `ROBOT001` | `RobotsTxtAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Path allowed by robots.txt override |
| `ROBOT002` | `RobotsTxtAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Path disallowed by robots.txt |
| `ROBOT003` | `RobotsTxtAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | High crawl-delay value |
| `ROBOT004` | `RobotsTxtAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Invalid sitemap URL in robots.txt |
| `ROBOTS-D001` | `RobotsTxtDirectivesAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Page disallowed by robots.txt |
| `ROBOTS-V3001` | `RobotsTxtAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No robots.txt |
| `ROBOTS-V3002` | `RobotsTxtAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt blocks all |
| `ROBOTS001` | `RobotsMetaAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | noindex on content page |
| `ROBOTS002` | `RobotsMetaAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | nofollow on content page |
| `ROBOTS003` | `RobotsMetaAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Conflicting robots directives |
| `ROBOTSDEEP-V2001` | `RobotsTxtAnalysisDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt blocks all |
| `ROBOTSDEEP-V2001-DEEP-DEEP` | `RobotsTxtAnalysisDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing User-agent in robots.txt (deep-deep) |
| `ROBOTSDEEP001` | `RobotsTxtAnalysisDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | robots.txt blocks all crawlers |
| `ROBOTSDEEP002` | `RobotsTxtAnalysisDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | High crawl-delay value |
| `ROBOTSDEPTH-V6100` | `RobotsTxtDisallowDepthValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Deep disallow paths in robots.txt |
| `ROBOTSDIS-V5001` | `RobotsTxtDisallowValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt blocks all crawlers |
| `ROBOTSDIS-V5002` | `RobotsTxtDisallowValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt has many Disallow rules |
| `ROBOTSEMPTY-V2001` | `RobotsTxtEmptyDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Empty robots.txt (deep-deep) |
| `ROBOTSEMPTY-V6099` | `RobotsTxtEmptyValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Empty robots.txt |
| `ROBOTSEMPTY001` | `RobotsTxtEmptyDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Empty robots.txt |
| `ROBOTSSIZE-V2001` | `RobotsTxtSizeValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt very large |
| `ROBOTSUA-V6102` | `RobotsTxtMissingUserAgentValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt missing User-agent |
| `ROBOTSWILD-V6101` | `RobotsTxtWildcardDisallowValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | robots.txt blocks all crawlers |
| `RP-V5001` | `ReferrerPolicyValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Referrer-Policy |
| `RP-V5002` | `ReferrerPolicyValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Referrer-Policy unsafe-url |
| `RP-V5003` | `ReferrerPolicyValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Referrer-Policy no-referrer |
| `RPABOUT-V2001` | `ResearchProjectMissingAboutValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | ResearchProject missing about |
| `RPABOUT-V6044` | `ResearchProjectMissingAboutValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | ResearchProject missing about |
| `RPDEEP-V2001` | `ReferrerPolicyDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing Referrer-Policy |
| `RPDEEP-V2002` | `ReferrerPolicyDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Referrer-Policy unsafe-url |
| `RPDEEP001` | `ReferrerPolicyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Missing Referrer-Policy header |
| `RPDEEP002` | `ReferrerPolicyDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Referrer-Policy set to unsafe-url |
| `RPFUND-V6045` | `ResearchProjectMissingFunderValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | ResearchProject missing funder |
| `RPFUND001` | `ResearchProjectMissingFundingRecognizerValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | ResearchProject missing funding info |
| `RPROJ-V2001` | `ResearchProjectSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/research_project_schema_v2.rs` | ResearchProject schema missing about |
| `RPROJ001` | `ResearchProjectSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/research_project_schema.rs` | ResearchProject schema missing name |
| `RPROJ002` | `ResearchProjectSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/research_project_schema.rs` | ResearchProject schema missing about |
| `RPSTRICT-V2001` | `ReferrerPolicyStrictDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Referrer-Policy not using strict policy (deep-deep) |
| `RPSTRICT-V6071` | `ReferrerPolicyStrictValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Referrer-Policy too permissive |
| `RPSTRICT001` | `ReferrerPolicyStrictDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Referrer-Policy not using strict policy |
| `SASPORT-V6009` | `SportsActivityLocationMissingSportValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | SportsActivityLocation missing sport |
| `SCHED-V2001` | `ScheduleSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/schedule_schema_v2.rs` | Schedule schema missing scheduleTimezone |
| `SCHED001` | `ScheduleSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/schedule_schema.rs` | Schedule schema missing name |
| `SCHED002` | `ScheduleSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/schedule_schema.rs` | Schedule schema missing scheduleTimezone |
| `SCHEDFREQ001` | `ScheduleMissingRepeatFrequencyValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Schedule missing repeatFrequency |
| `SCHEDTZ-V2001` | `ScheduleMissingTimezoneValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Schedule missing timezone |
| `SCHEDTZ-V6046` | `ScheduleMissingTimezoneValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Schedule missing scheduleTimezone |
| `SCHEMACOV001` | `SchemaCoverageScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | No structured data present |
| `SCHEMACOV002` | `SchemaCoverageScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Non-standard @context |
| `SCRIPT001` | `ScriptAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Excessive script count |
| `SCRIPT002` | `ScriptAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Multiple blocking scripts |
| `SCRIPTBLK-V5001` | `ScriptAsyncDeferValidator` | `crates/crawlkit-engine/src/analyzers/v2/performance.rs` | Blocking scripts detected |
| `SD001` | `StructuredDataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | No structured data found |
| `SD002` | `StructuredDataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Missing @context |
| `SD003` | `StructuredDataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Non-standard @context |
| `SD004` | `StructuredDataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Missing @type |
| `SD005` | `StructuredDataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Unknown @type |
| `SD006` | `StructuredDataValidator` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | (no title literal) |
| `SEC012` | `SecurityHeaderAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_aggregator_analyzers.rs` | Security posture score |
| `SECSC001` | `SecurityScoreAnalyzer` | `crates/crawlkit-engine/src/analyzers/v2/scoring.rs` | Security header score |
| `SENAME-V6016` | `SportsEventMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | SportsEvent missing name |
| `SERVER001` | `ServerHeaderAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Server header leaks version information |
| `SERVER002` | `ServerHeaderAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Server header reveals technology stack |
| `SESPORT-V6015` | `SportsEventMissingSportValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | SportsEvent missing sport |
| `SHIP001` | `ShippingSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/shipping_schema.rs` | Product has offers but no ShippingDetails |
| `SITEMAP-U001` | `SitemapUrlAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | URL contains query parameters |
| `SITEMAP-U002` | `SitemapUrlAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | URL contains uppercase characters |
| `SITEMAP-V3001` | `SitemapAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No robots.txt |
| `SITEMAP002` | `SitemapAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Page not found in sitemap |
| `SITEMAP003` | `SitemapAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Invalid lastmod format |
| `SITEMAP004` | `SitemapAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Invalid changefreq value |
| `SITEMAP005` | `SitemapAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Invalid priority value |
| `SITEMAP006` | `SitemapCanonicalValidator` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Non-canonical page with canonical tag |
| `SITEMAPCOV-V5001` | `SitemapCoverageValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No sitemap in robots.txt |
| `SITEMAPDEEP-V2001` | `SitemapCoverageDeepAnalyzerV2`, `SitemapCoverageDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No sitemap in robots.txt |
| `SITEMAPDEEP001` | `SitemapCoverageDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Many sitemaps declared in robots.txt |
| `SITEMAPDEEP002` | `SitemapCoverageDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | No sitemap declared in robots.txt |
| `SITEMAPLMFMT-V6097` | `SitemapLastmodFormatValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Sitemap lastmod format issues |
| `SITEMAPMISS-V2001` | `SitemapMissingDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No sitemap reference (deep-deep) |
| `SITEMAPMISS-V6096` | `SitemapMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No sitemap in robots.txt |
| `SITEMAPMISS001` | `SitemapMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | No sitemap in robots.txt |
| `SITEMAPMOD-V5001` | `SitemapLastmodValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Sitemap missing lastmod |
| `SITEMAPPRI-V5001` | `SitemapPriorityValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Sitemap missing priority |
| `SITEMAPPRI-V6098` | `SitemapPriorityRangeValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Sitemap priority out of range |
| `SITEMAPSIZE-V2001` | `SitemapXmlSizeValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Sitemap very large |
| `SITEMAPSIZE-V2003` | `SitemapXmlSizeValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Sitemap exceeds URL limit |
| `SIZE001` | `ResponseSizeAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Response body exceeds 5MB |
| `SIZE002` | `ResponseSizeAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Response body exceeds 10MB |
| `SKIPLINK001` | `SkipLinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/skip_link_analyzers.rs` | No skip navigation link |
| `SOCIAL001` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG image missing dimensions |
| `SOCIAL002` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG image too narrow |
| `SOCIAL003` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG image too short |
| `SOCIAL004` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Missing Twitter Card type |
| `SOCIAL005` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Invalid Twitter Card type |
| `SOCIAL006` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Incomplete Open Graph tags |
| `SOCIAL007` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Incomplete Twitter Card tags |
| `SOCIAL008` | `SocialMediaAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Social preview completeness score |
| `SOFT001` | `SoftwareApplicationValidator` | `crates/crawlkit-engine/src/analyzers/schema/software_application.rs` | SoftwareApplication missing operatingSystem |
| `SOFT002` | `SoftwareApplicationValidator` | `crates/crawlkit-engine/src/analyzers/schema/software_application.rs` | SoftwareApplication missing offers |
| `SOFTOFF-V5001` | `SoftwareOffersValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Software missing offers |
| `SOFTSS-V5001` | `SoftwareScreenshotValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Software missing screenshot |
| `SPEAK001` | `SpeakableSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/speakable_schema.rs` | Speakable schema missing xpath |
| `SPEAK002` | `SpeakableSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/speakable_schema.rs` | Speakable schema missing cssSelector |
| `SPEC001` | `SpecialAnnouncementSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/special_announcement_schema.rs` | SpecialAnnouncement missing datePosted |
| `SPEC002` | `SpecialAnnouncementSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/special_announcement_schema.rs` | SpecialAnnouncement missing category |
| `SPORTS001` | `SportsActivityLocationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/sports_activity_location_schema.rs` | SportsActivityLocation schema missing name |
| `SPORTS002` | `SportsActivityLocationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/sports_activity_location_schema.rs` | SportsActivityLocation schema missing address |
| `SPORTSEVT001` | `SportsEventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/sports_event_schema.rs` | SportsEvent schema missing name |
| `SPORTSEVT002` | `SportsEventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/sports_event_schema.rs` | SportsEvent schema missing startDate |
| `SPORTSEVT003` | `SportsEventSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/sports_event_schema.rs` | SportsEvent schema missing location |
| `SPREV001` | `SocialPreviewOptimizer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG title missing |
| `SPREV002` | `SocialPreviewOptimizer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG description missing |
| `SPREV003` | `SocialPreviewOptimizer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | OG image URL invalid |
| `SRI-V5001` | `SriValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | No SRI on external scripts |
| `SRI001` | `SriAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | External scripts missing integrity attribute |
| `SRI002` | `SriAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | External stylesheets missing integrity attribute |
| `SRISCRIPT001` | `SubresourceIntegrityAnalyzer` | `crates/crawlkit-engine/src/analyzers/dns_sri_cors_analyzers.rs` | External scripts missing Subresource Integrity |
| `SSL000` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | SSL certificate not inspected |
| `SSL001` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | SSL certificate has expired |
| `SSL002` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | SSL certificate expiring soon |
| `SSL003` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Invalid certificate chain |
| `SSL004` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Subject/SAN does not match hostname |
| `SSL005` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Weak signature algorithm |
| `SSL006` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Self-signed certificate detected |
| `SSL008` | `SslCertificateValidator` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | SSL certificate details |
| `STRICT001` | `StrictTransportSecurityAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | Missing Strict-Transport-Security header |
| `STRICT002` | `StrictTransportSecurityAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | HSTS max-age is too short |
| `STYLE001` | `StylesheetAnalyzer` | `crates/crawlkit-engine/src/analyzers/media_analyzers.rs` | Excessive stylesheet count |
| `SVC-V2001` | `ServiceSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/service_schema_v2.rs` | Service schema missing areaServed |
| `SVC001` | `ServiceSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/service_schema.rs` | Service schema missing name |
| `SVC002` | `ServiceSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/service_schema.rs` | Service schema missing provider |
| `SVCAREA-V6034` | `ServiceMissingAreaServedValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Service missing areaServed |
| `SVCPRV-V2001` | `ServiceMissingProviderValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Service missing provider |
| `SVCPRV-V6035` | `ServiceMissingProviderValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Service missing provider |
| `SVCTYPE-V6057` | `ServiceMissingServiceTypeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Service missing serviceType |
| `TAB-V2001` | `TabindexAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Positive tabindex values found |
| `TABACC-V2001` | `TableAccessibilityDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables without headers |
| `TABACC-V2001-DEEP-DEEP` | `TableAccessibilityDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables without headers (deep-deep) |
| `TABACC-V2001-DEEP-DEEP-DEEP` | `TableAccessibilityDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables without headers (deep-deep-deep) |
| `TABACC-V2002` | `TableAccessibilityDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables without captions |
| `TABACC-V2002-DEEP-DEEP-DEEP` | `TableAccessibilityDeepDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables without captions (deep-deep-deep) |
| `TABACCDEEP001` | `TableAccessibilityDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Tables without headers |
| `TABACCDEEP002` | `TableAccessibilityDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/accessibility_deep_analyzers.rs` | Tables missing captions |
| `TABCAP-V6112` | `TableCaptionMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables missing captions |
| `TABINDEX001` | `TabindexAnalyzer` | `crates/crawlkit-engine/src/analyzers/tabindex_analyzers.rs` | Positive tabindex values detected |
| `TABLECAP001` | `TableCaptionAnalyzer` | `crates/crawlkit-engine/src/analyzers/table_caption_analyzers.rs` | Table missing caption element |
| `TABPOS001` | `FocusOrderPositiveTabindexAnalyzer` | `crates/crawlkit-engine/src/analyzers/aria_focus_link_validator_analyzers.rs` | Positive tabindex values disrupt focus order |
| `TABSCOPE-V6111` | `TableHeadersScopeValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables missing header scope |
| `TACC001` | `TableAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/table_analyzers.rs` | Table missing header cells |
| `TACC002` | `TableAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/table_analyzers.rs` | Table missing caption |
| `TACC003` | `TableAccessibilityAnalyzer` | `crates/crawlkit-engine/src/analyzers/table_analyzers.rs` | Large number of tables missing scope attributes |
| `TANAME-V6013` | `TouristAttractionMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | TouristAttraction missing name |
| `TBL-V2001` | `TableAccessibilityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/accessibility_v2_analyzers.rs` | Tables without headers |
| `TBLCAP-V2001` | `TableCaptionPresenceAnalyzerV2`, `TableCaptionPresenceDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | No table captions |
| `TBLCAP001` | `TableCaptionPresenceAnalyzer` | `crates/crawlkit-engine/src/analyzers/form_table_validator_analyzers.rs` | Tables missing caption element |
| `TBLCAPT-V5001` | `TableCaptionValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Tables missing captions |
| `TBLHDR-V5001` | `TableHeadersValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | All tables missing headers |
| `TBLHDR-V5002` | `TableHeadersValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Some tables missing headers |
| `TBLSCOP-V2001` | `TableHeaderScopeAnalyzerV2`, `TableHeaderScopeDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Table headers missing scope |
| `TBLSCOP001` | `TableHeaderScopeAnalyzer` | `crates/crawlkit-engine/src/analyzers/form_table_validator_analyzers.rs` | Tables missing header cells with scope |
| `TBLSCOPE-V5001` | `TableScopeValidator` | `crates/crawlkit-engine/src/analyzers/v2/accessibility.rs` | Table headers missing scope |
| `TDNAME-V6014` | `TouristDestinationMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | TouristDestination missing name |
| `TITLE-PX001` | `TitlePixelWidthAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title exceeds SERP pixel width |
| `TITLE-PX002` | `TitlePixelWidthAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title too short for SERP display |
| `TITLE-V4001` | `TitleAnalyzerV4` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing title tag |
| `TITLE-V4002` | `TitleAnalyzerV4` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title too short |
| `TITLE-V4003` | `TitleAnalyzerV4` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title may truncate |
| `TITLE-V4004` | `TitleAnalyzerV4` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | ALL CAPS title |
| `TITLEBRAND-V2001` | `TitleBrandPlacementDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title ends with separator (deep-deep) |
| `TITLEBRAND-V5001` | `TitleBrandValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title missing brand name |
| `TITLEBRAND-V6083` | `TitleBrandPlacementValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Brand not at start of title |
| `TITLEBRAND001` | `TitleBrandPlacementDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title ends with separator |
| `TITLEDEEP-V2001` | `TitleAnalysisDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title critically short |
| `TITLEDEEP-V2002` | `TitleAnalysisDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title excessively long |
| `TITLEDEEP-V2003` | `TitleAnalysisDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title all lowercase or uppercase |
| `TITLEDEEP001` | `TitleAnalysisDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title too short |
| `TITLEDEEP002` | `TitleAnalysisDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title too long for SERP display |
| `TITLEDEEP003` | `TitleAnalysisDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title missing brand separator |
| `TITLEDEEP004` | `TitleAnalysisDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Title has many stop words |
| `TITLEKDEN-V2001` | `TitleKeywordDensityDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Low title-description keyword overlap (deep-deep) |
| `TITLEKDEN-V6082` | `TitleKeywordDensityValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title keyword density low |
| `TITLEKDEN001` | `TitleKeywordDensityDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Low title-description keyword overlap |
| `TITLEKWP-V5001` | `TitleKeywordPresenceValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title missing description keywords |
| `TITLELEN-V2001` | `TitleLengthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title too long (deep) |
| `TITLELEN-V2002` | `TitleLengthDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title too short (deep) |
| `TITLELEN-V5001` | `TitleLengthValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing title tag |
| `TITLELEN-V5002` | `TitleLengthValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title too short |
| `TITLELEN-V5003` | `TitleLengthValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title may truncate |
| `TITLEMISS-V2001` | `TitleMissingDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing title tag (deep-deep) |
| `TITLEMISS-V6081` | `TitleMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing title tag |
| `TITLEMISS001` | `TitleMissingDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Missing title tag |
| `TITLEPX-V6084` | `TitlePixelWidthValidator` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title may be truncated in SERPs |
| `TITLEQLT-V2001` | `TitleLengthQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title too short |
| `TITLEQLT-V2002` | `TitleLengthQualityAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Title may truncate |
| `TOC001` | `TableOfContentsAnalyzer` | `crates/crawlkit-engine/src/analyzers/content_analyzers.rs` | Long-form page missing table of contents |
| `TOURIST001` | `TouristAttractionSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tourist_attraction_schema.rs` | TouristAttraction schema missing name |
| `TOURIST002` | `TouristAttractionSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tourist_attraction_schema.rs` | TouristAttraction schema missing location |
| `TOURIST003` | `TouristAttractionSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tourist_attraction_schema.rs` | TouristAttraction schema missing touristType |
| `TOURISTDEST001` | `TouristDestinationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tourist_destination_schema.rs` | TouristDestination schema missing name |
| `TOURISTDEST002` | `TouristDestinationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tourist_destination_schema.rs` | TouristDestination schema missing location |
| `TOURISTDEST003` | `TouristDestinationSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tourist_destination_schema.rs` | TouristDestination schema missing touristType |
| `TRIP-V2001` | `TripSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/trip_schema_v2.rs` | Trip schema missing itinerary |
| `TRIP001` | `TripSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/trip_schema.rs` | Trip schema missing name |
| `TRIP002` | `TripSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/trip_schema.rs` | Trip schema missing itinerary |
| `TRIPDEP001` | `TripMissingDepartureTimeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Trip missing departureTime |
| `TRIPIT-V2001` | `TripMissingItineraryValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Trip missing itinerary |
| `TRIPIT-V6047` | `TripMissingItineraryValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Trip missing itinerary |
| `TTFB001` | `TtfbAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | Slow Time to First Byte (TTFB) |
| `TTFB002` | `TtfbAnalyzer` | `crates/crawlkit-engine/src/analyzers/http_analyzers.rs` | High Time to First Byte (TTFB) |
| `TV-V2001` | `TVSeriesSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/tv_series_schema_v2.rs` | TVSeries schema missing numberOfEpisodes |
| `TV001` | `TVSeriesSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tv_series_schema.rs` | TVSeries schema missing name |
| `TV002` | `TVSeriesSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tv_series_schema.rs` | TVSeries schema missing numberOfEpisodes |
| `TVEP-V2001` | `TVSeriesMissingEpisodesValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | TVSeries missing episodes |
| `TVEP-V6031` | `TVSeriesMissingEpisodeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | TVSeries missing episode |
| `TVEP001` | `TVSeriesEpisodeSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/tv_series_episode_schema.rs` | TVEpisode schema missing episodeNumber |
| `TVEPISODES001` | `TVSeriesMissingNumberOfEpisodesValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | TVSeries missing numberOfEpisodes |
| `TVSEASON-V6030` | `TVSeriesMissingNumberOfSeasonsValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | TVSeries missing numberOfSeasons |
| `TW001` | `TwitterCardTypeAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Twitter card type missing |
| `TW002` | `TwitterCardTypeAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Twitter card type could be summary_large_image |
| `TWPL001` | `TwitterPlayerValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | twitter:player:stream missing |
| `TWPL002` | `TwitterPlayerValidator` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | twitter:player:stream missing dimensions |
| `TWSITE001` | `TwitterSiteAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | Missing twitter:site tag |
| `TWSITE002` | `TwitterSiteAnalyzer` | `crates/crawlkit-engine/src/analyzers/social_analyzers.rs` | twitter:site does not start with @ |
| `URL001` | `AdvancedCanonicalAnalyzer` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Double slash in URL path |
| `URL002` | `UrlFormatValidator` | `crates/crawlkit-engine/src/advanced_canonical.rs` | Uppercase characters in URL |
| `VID001` | `VideoSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_schema.rs` | VideoObject missing embedUrl |
| `VID002` | `VideoSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_schema.rs` | VideoObject missing thumbnailUrl |
| `VID003` | `VideoSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_schema.rs` | VideoObject missing duration |
| `VIDDUR001` | `VideoObjectDurationValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_object_duration.rs` | VideoObject missing duration |
| `VIDDUR002` | `VideoObjectDurationValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_object_duration.rs` | VideoObject duration is not valid ISO 8601 |
| `VIDDURFMT-V5001` | `VideoDurationFormatValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Video duration not ISO 8601 |
| `VIDEMB001` | `VideoObjectEmbedUrlValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_object_embed_url.rs` | VideoObject missing embedUrl |
| `VIDEMB002` | `VideoObjectEmbedUrlValidator` | `crates/crawlkit-engine/src/analyzers/schema/video_object_embed_url.rs` | VideoObject missing contentUrl |
| `VIDTHUMB-V5001` | `VideoThumbnailValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Video missing thumbnailUrl |
| `WAPI-V2001` | `WebAPISchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/web_api_schema_v2.rs` | WebAPI schema missing documentation |
| `WAPI001` | `WebAPISchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_api_schema.rs` | WebAPI schema missing name |
| `WAPI002` | `WebAPISchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_api_schema.rs` | WebAPI schema missing documentation |
| `WAPIDOC-V2001` | `WebAPIMissingDocumentationValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WebAPI missing documentation |
| `WAPIDOC-V6049` | `WebAPIMissingDocumentationValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WebAPI missing documentation |
| `WAPIDOCS001` | `WebAPIDocumentationMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WebAPI missing documentation |
| `WASM001` | `WasmPatternAnalyzer` | `crates/crawlkit-engine/src/wasm_analyzers.rs` | Missing WASM module preload |
| `WASM002` | `WasmPatternAnalyzer` | `crates/crawlkit-engine/src/wasm_analyzers.rs` | Synchronous WASM compilation detected |
| `WASM003` | `WasmPatternAnalyzer` | `crates/crawlkit-engine/src/wasm_analyzers.rs` | WASM instantiation without error handling |
| `WC001` | `WordCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Word count statistics |
| `WC002` | `WordCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Zero word count |
| `WC003` | `WordCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Very low word count |
| `WC004` | `WordCountAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Long average sentence length |
| `WEAR-V2001` | `WearableSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/wearable_schema_v2.rs` | WearableDevice schema missing deviceType |
| `WEAR001` | `WearableSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/wearable_schema.rs` | Wearable schema missing name |
| `WEAR002` | `WearableSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/wearable_schema.rs` | Wearable schema missing deviceType |
| `WEARBATT001` | `WearableMissingBatteryLifeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Wearable missing battery info |
| `WEARDEV-V2001` | `WearableMissingDeviceTypeValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Wearable missing deviceType |
| `WEARDEV-V6050` | `WearableMissingDeviceTypeValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Wearable missing deviceType |
| `WEBPG001` | `WebPageSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_page_schema.rs` | WebPage schema missing name |
| `WEBPG002` | `WebPageSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_page_schema.rs` | WebPage schema missing datePublished |
| `WELEM-V2001` | `WebPageElementSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/web_page_element_schema_v2.rs` | WebPageElement schema missing name |
| `WELEM001` | `WebPageElementSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_page_element_schema.rs` | WebPageElement schema missing name |
| `WIKI-V3001` | `WikipediaLinkAnalyzerV3` | `crates/crawlkit-engine/src/analyzers/v2/seo.rs` | Wikipedia links found |
| `WIKI001` | `WikipediaLinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Wikipedia links detected |
| `WIKI002` | `WikipediaLinkAnalyzer` | `crates/crawlkit-engine/src/analyzers/seo_analyzers.rs` | Wikidata links detected |
| `WOCCUP001` | `WorkerMissingOccupationValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Worker missing occupation/jobTitle |
| `WORKER-V2001` | `WorkerSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/worker_schema_v2.rs` | Worker schema missing jobTitle |
| `WORKER001` | `WorkerSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/worker_schema.rs` | Worker schema missing name |
| `WORKER002` | `WorkerSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/worker_schema.rs` | Worker schema missing jobTitle |
| `WORKJOB-V2001` | `WorkerMissingJobTitleValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Worker missing jobTitle |
| `WORKJOB-V6052` | `WorkerMissingJobTitleValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | Worker missing jobTitle |
| `WPELACC001` | `WebPageElementMissingAccessibleNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WebPageElement missing name |
| `WPELNAME-V2001` | `WebPageElementMissingNameValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WebPageElement missing name |
| `WPELNAME-V6051` | `WebPageElementMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WebPageElement missing name |
| `WSITE-V2001` | `WebSiteSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/web_site_schema_v2.rs` | WebSite schema missing url |
| `WSITE001` | `WebSiteSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_site_schema.rs` | WebSite schema missing name |
| `WSITE002` | `WebSiteSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/web_site_schema.rs` | WebSite schema missing url |
| `WUMEMBER001` | `WorkersUnionMissingMemberCountValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WorkersUnion missing memberCount |
| `WUNAME-V2001` | `WorkersUnionMissingNameValidatorV2` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WorkersUnion missing name |
| `WUNAME-V6048` | `WorkersUnionMissingNameValidator` | `crates/crawlkit-engine/src/analyzers/v2/schema.rs` | WorkersUnion missing name |
| `WUNION-V2001` | `WorkersUnionSchemaValidatorV2` | `crates/crawlkit-engine/src/analyzers/schema/workers_union_schema_v2.rs` | WorkersUnion schema missing name |
| `WUNION001` | `WorkersUnionSchemaValidator` | `crates/crawlkit-engine/src/analyzers/schema/workers_union_schema.rs` | WorkersUnion schema missing name |
| `XCTO-V2001` | `XContentTypeOptionsAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | Missing X-Content-Type-Options header |
| `XCTO-V2001-DEEP` | `XContentTypeOptionsDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing X-Content-Type-Options |
| `XCTO-V2002` | `XContentTypeOptionsDeepAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Invalid X-Content-Type-Options |
| `XCTO-V5001` | `XContentTypeOptionsValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Missing X-Content-Type-Options |
| `XCTO-V5002` | `XContentTypeOptionsValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Invalid X-Content-Type-Options |
| `XCTO001` | `XContentTypeOptionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/x_header_analyzers.rs` | Missing X-Content-Type-Options header |
| `XCTO002` | `XContentTypeOptionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/x_header_analyzers.rs` | X-Content-Type-Options not set to nosniff |
| `XCTODEEP001` | `XContentTypeOptionsDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Missing X-Content-Type-Options header |
| `XCTODEEP002` | `XContentTypeOptionsDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Invalid X-Content-Type-Options value |
| `XFO-V2001` | `XFrameOptionsAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v3_analyzers.rs` | Missing X-Frame-Options header |
| `XFO-V5001` | `XFrameOptionsValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | No clickjacking protection |
| `XFO-V5002` | `XFrameOptionsValidatorV5` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | Invalid X-Frame-Options value |
| `XFO001` | `XFrameOptionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | Missing X-Frame-Options header on HTML page |
| `XFO002` | `XFrameOptionsAnalyzer` | `crates/crawlkit-engine/src/analyzers/security_header_analyzers.rs` | X-Frame-Options set to ALLOWALL |
| `XFODEEP-V2001` | `XFrameOptionsDeepAnalyzerV2`, `XFrameOptionsDeepDeepValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | No clickjacking protection |
| `XFODEEP001` | `XFrameOptionsDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | No clickjacking protection |
| `XFODEEP002` | `XFrameOptionsDeepAnalyzer` | `crates/crawlkit-engine/src/analyzers/deep_security_header_analyzers.rs` | Invalid X-Frame-Options value |
| `XFOMISS-V6072` | `XFrameOptionsMissingValidator` | `crates/crawlkit-engine/src/analyzers/v2/security.rs` | No clickjacking protection |
| `XPCDP002` | `XPermittedCrossDomainPoliciesAnalyzer` | `crates/crawlkit-engine/src/analyzers/x_header_analyzers.rs` | X-Permitted-Cross-Domain-Policies set to all |
| `XSS-V2001` | `XssProtectionAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | Missing X-XSS-Protection header |
| `XSS-V2002` | `XssProtectionAnalyzerV2` | `crates/crawlkit-engine/src/analyzers/security_header_v2_analyzers.rs` | X-XSS-Protection explicitly disabled |
| `XSS002` | `XSSProtectionAnalyzer` | `crates/crawlkit-engine/src/analyzers/sts_analyzers.rs` | (no title literal) |
