# Analyzer Catalog

**Total analyzers:** 200 (181 single-page + 19 cross-page)
**Generated:** 2026-08-27

---

## Single-Page Analyzers (181)

### HTTP (7)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 1 | HTTP Status | `HttpStatusAnalyzer` | HTTP001–HTTP007 | Critical/Error/Warning/Info | Detects broken pages, server errors, soft 404s, and response time issues |
| 2 | Redirect Chain | `RedirectChainAnalyzer` | REDIR001–REDIR004 | Critical/Warning/Info | Identifies long redirect chains, loops, and mixed-protocol redirects that waste crawl budget |
| 3 | HTTP Version | `HttpVersionAnalyzer` | HTTPVER001–HTTPVER002 | Warning/Info | Detects outdated HTTP/1.0 or HTTP/1.1 responses when newer protocols are available |
| 4 | Server Header | `ServerHeaderAnalyzer` | SERVER001–SERVER002 | Warning/Info | Flags server version and technology stack leaks that aid attackers |
| 5 | Compression | `CompressionAnalyzer` | COMP001–COMP002 | Warning/Info | Detects uncompressed responses and unnecessary compression overhead |
| 6 | Response Size | `ResponseSizeAnalyzer` | SIZE001–SIZE003 | Error/Warning/Info | Flags oversized responses (>5MB warning, >10MB error) and missing Content-Length |
| 7 | TTFB | `TtfbAnalyzer` | TTFB001–TTFB002 | Error/Warning | Measures Time to First Byte and flags slow server responses (>600ms warning, >1000ms error) |

### SEO (30)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 8 | Canonical URL | `CanonicalUrlValidator` | CANON001–CANON003 | Warning/Info | Validates canonical tag presence and detects mismatches that confuse search engines |
| 9 | Hreflang | `HreflangValidator` | HREF001–HREF003 | Warning/Error | Validates hreflang tags for x-default, locale code format, and duplicate languages |
| 10 | Sitemap | `SitemapAnalyzer` | SITEMAP002–SITEMAP005 | Warning | Checks if pages are in sitemaps and validates lastmod, changefreq, priority formats |
| 11 | Robots.txt | `RobotsTxtAnalyzer` | ROBOT001–ROBOT004 | Error/Warning/Info | Validates robots.txt rules, crawl-delay values, and sitemap references |
| 12 | Meta Tags | `MetaTagAnalyzer` | META001–META006, META009 | Error/Warning | Checks title and meta description length/absence, and viewport tag presence |
| 13 | Heading Hierarchy | `HeadingHierarchyAnalyzer` | HEAD001–HEAD005 | Error/Warning/Info | Validates heading structure for H1 presence, count, and skipped levels |
| 14 | Link Analyzer | `LinkAnalyzer` | LINK001–LINK006 | Error/Warning/Info | Analyzes internal/external links, nofollow usage, anchor text quality, and orphan pages |
| 15 | OpenSearch | `OpenSearchValidator` | OPSEARCH001 | Info | Checks for OpenSearch description XML link for browser search integration |
| 16 | Word Count | `WordCountAnalyzer` | WC001–WC004 | Warning/Info | Measures word count, sentence length, and flags thin content (<100 words) |
| 17 | Keyword Analyzer | `KeywordAnalyzer` | KW001–KW004 | Warning/Info | Computes TF-IDF scores, keyword density, prominence, and co-occurrence |
| 18 | International SEO | `InternationalSeoAnalyzer` | ISEO001–ISEO006 | Warning/Info | Validates hreflang-URL locale consistency, URL locale segments, and cross-page hreflang |
| 19 | Advanced Canonical | `AdvancedCanonicalAnalyzer` | CANON004, ISEO007, URL001 | Warning | Detects canonical pointing to redirects, hreflang to non-canonical, double slashes |
| 20 | Sitemap Canonical | `SitemapCanonicalValidator` | SITEMAP006 | Warning | Flags non-canonical pages with canonical tags pointing elsewhere |
| 21 | URL Format | `UrlFormatValidator` | URL002 | Info | Detects uppercase characters in URL paths that cause case-sensitivity issues |
| 22 | Pagination | `PaginationAnalyzer` | PAGIN001–PAGIN003 | Warning/Info | Validates rel=next/prev pagination chains and page parameter consistency |
| 23 | Internal Link Anchor | `InternalLinkAnchorAnalyzer` | ILINK001–ILINK003 | Warning/Info | Analyzes internal link anchor text quality and distribution |
| 24 | Wikipedia Link | `WikipediaLinkAnalyzer` | WIKI001 | Info | Detects Wikipedia/Wikidata links that boost E-E-A-T signals |
| 25 | Anchor Text Diversity | `AnchorTextDiversityAnalyzer` | ANCHOR001–ANCHOR002 | Warning/Info | Measures anchor text diversity to detect over-optimization |
| 26 | Robots Meta | `RobotsMetaAnalyzer` | ROBOTS001–ROBOTS002 | Warning/Info | Validates meta robots directives for noindex, nofollow consistency |
| 27 | Canonical Depth | `CanonicalDepthAnalyzer` | CANON006 | Info | Flags canonical URLs that are too deep in the site hierarchy |
| 28 | Hreflang Consistency | `HreflangConsistencyAnalyzer` | HREFC001–HREFC002 | Warning/Info | Checks cross-page hreflang reciprocal references and language code consistency |
| 29 | Charset | `CharsetValidator` | CHARSET001–CHARSET002 | Warning/Info | Validates charset declaration presence and UTF-8 encoding |
| 30 | Robots.txt Directives | `RobotsTxtDirectivesAnalyzer` | RTDIR001–RTDIR002 | Warning/Info | Analyzes robots.txt directive quality and sitemap references |
| 31 | Sitemap URL | `SitemapUrlAnalyzer` | SMURL001–SMURL002 | Warning/Info | Validates sitemap URL format and checks for unreachable URLs |
| 32 | Language Attribute | `LanguageAttributeAnalyzer` | LANG001–LANG002 | Error/Warning | Validates html lang attribute presence and matches with hreflang declarations |
| 33 | Mobile Viewport | `MobileViewportAnalyzer` | VIEW001–VIEW002 | Warning/Info | Checks viewport meta tag configuration for mobile responsiveness |

### Content (19)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 34 | Content Quality | `ContentQualityAnalyzer` | CQ001–CQ005 | Warning/Info | Measures readability scores, keyword density, and content-to-markup ratio |
| 35 | Entity Analyzer | `EntityAnalyzer` | ENTITY001–ENTITY006 | Info | Extracts people, organizations, locations, topics, and sentiment from content |
| 36 | Enhanced Readability | `EnhancedReadabilityAnalyzer` | READ001–READ005 | Info | Computes Flesch-Kincaid, Coleman-Liau, ARI, Gunning Fog, and Reading Ease scores |
| 37 | Structured Data | `StructuredDataValidator` | SD001–SD006 | Error/Warning/Info | Validates JSON-LD @context, @type, and required properties per Schema.org type |
| 38 | RDFa Validator | `RdfaValidator` | RDFA001–RDFA003 | Error/Warning | Validates RDFa attributes for vocab, typeof, and deprecated vocabulary usage |
| 39 | Microdata Validator | `MicrodataValidator` | MD001–MD003 | Error/Warning | Validates Microdata itemscope/itemprop and checks for known Schema.org types |
| 40 | Entity Linking | `EntityLinkingAnalyzer` | ELINK001–ELINK003 | Warning/Info | Checks entity links to Wikipedia and cross-entity references in structured data |
| 41 | Duplicate Content | `DuplicateContentDetector` | DUP001–DUP003 | Warning/Info | Detects near-duplicate titles, descriptions, and content across pages |
| 42 | Table of Contents | `TableOfContentsAnalyzer` | TOC001–TOC002 | Warning/Info | Checks for heading-based table of contents and content structure |
| 43 | Meta Description Length | `MetaDescriptionLengthAnalyzer` | META010–META011 | Warning | Validates meta description length (120–160 characters optimal) |
| 44 | Title Length | `TitleLengthAnalyzer` | TITLE001–TITLE002 | Warning | Validates title tag length (30–60 characters optimal) |
| 45 | Content Thin | `ContentThinAnalyzer` | THIN001–THIN002 | Warning | Detects thin content pages with insufficient word count |
| 46 | Content Freshness | `ContentFreshnessScorer` | FRESH001–FRESH002 | Info | Scores content freshness based on dates, update signals, and recency |
| 47 | Breadcrumb List Depth | `BreadcrumbListDepthAnalyzer` | BREAD001–BREAD002 | Warning/Info | Validates breadcrumb schema depth and navigation hierarchy |
| 48 | JSON-LD Validator | `JsonLdValidator` | JLD001–JLD003 | Error/Warning | Validates JSON-LD syntax, @context, and @type completeness |
| 49 | Local Business NAP | `LocalBusinessNapAnalyzer` | NAP001–NAP002 | Warning/Info | Validates NAP (Name, Address, Phone) consistency in LocalBusiness schema |
| 50 | Shipping Schema | `ShippingSchemaValidator` | SHIP001–SHIP002 | Warning/Info | Validates shipping schema properties for e-commerce pages |
| 51 | Offer Availability | `OfferAvailabilityAnalyzer` | AVAIL001–AVAIL002 | Warning/Info | Validates offer availability status and schema correctness |
| 52 | Coupon Schema | `CouponSchemaValidator` | COUPON001–COUPON002 | Warning/Info | Validates coupon schema properties for discount offers |

### Schema (60)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 53 | Breadcrumbs | `BreadcrumbsValidator` | BREADSC001–BREADSC002 | Error/Warning | Validates BreadcrumbList schema for itemListElement and position properties |
| 54 | Event Schema | `EventSchemaValidator` | EVENT001–EVENT003 | Error/Warning | Validates Event schema for name, startDate, and location properties |
| 55 | Review Schema | `ReviewSchemaValidator` | REVIEW001–REVIEW002 | Warning/Info | Validates Review schema for author, reviewBody, and rating properties |
| 56 | Video Schema | `VideoSchemaValidator` | VIDEO001–VIDEO003 | Error/Warning | Validates VideoObject schema for name, embedUrl, and thumbnailUrl |
| 57 | Local Business | `LocalBusinessSchemaValidator` | LBIZ001–LBIZ003 | Error/Warning | Validates LocalBusiness schema for name, address, and geo properties |
| 58 | FAQ Schema | `FaqSchemaValidator` | FAQ001–FAQ003 | Error/Warning | Validates FAQPage schema for mainEntity with Question/Answer pairs |
| 59 | HowTo Schema | `HowToSchemaValidator` | HOWTO001–HOWTO003 | Error/Warning | Validates HowTo schema for name, step, and tool/supply properties |
| 60 | Speakable Schema | `SpeakableSchemaValidator` | SPEAK001–SPEAK002 | Warning/Info | Validates Speakable schema for voice-assistant-optimized content |
| 61 | Dataset Schema | `DatasetSchemaValidator` | DATASET001–DATASET003 | Error/Warning | Validates Dataset schema for name, description, and distribution |
| 62 | Special Announcement | `SpecialAnnouncementSchemaValidator` | SPECIAL001–SPECIAL002 | Warning/Info | Validates SpecialAnnouncement schema for timely public information |
| 63 | Software Application | `SoftwareApplicationValidator` | SWAPP001–SWAPP003 | Error/Warning | Validates SoftwareApplication schema for name, applicationCategory, and offers |
| 64 | Article Schema | `ArticleSchemaValidator` | ART001–ART003 | Error/Warning | Validates Article schema for headline, author, datePublished, and publisher |
| 65 | Organization Schema | `OrganizationSchemaValidator` | ORG001–ORG003 | Error/Warning | Validates Organization schema for name, url, and logo properties |
| 66 | Person Schema | `PersonSchemaValidator` | PERSON001–PERSON002 | Error/Warning | Validates Person schema for name and jobTitle properties |
| 67 | JobPosting Schema | `JobPostingSchemaValidator` | JOB001–JOB003 | Error/Warning | Validates JobPosting schema for title, datePosted, and hiringOrganization |
| 68 | Course Schema | `CourseSchemaValidator` | COURSE001–COURSE002 | Error/Warning | Validates Course schema for name, provider, and description |
| 69 | Recipe Schema | `RecipeSchemaValidator` | RECIPE001–RECIPE003 | Error/Warning | Validates Recipe schema for name, ingredients, and instructions |
| 70 | Product Variant | `ProductVariantAnalyzer` | PVAR001–PVAR002 | Warning/Info | Validates Product schema for hasVariant and offer availability |
| 71 | Pricing Schema | `PricingSchemaValidator` | PRICE001–PRICE002 | Error/Warning | Validates price, priceCurrency, and priceValidUntil in offers |
| 72 | Aggregate Rating | `AggregateRatingValidator` | AGRAT001–AGRAT002 | Error/Warning | Validates AggregateRating schema for ratingValue and reviewCount |
| 73 | WebPage Schema | `WebPageSchemaValidator` | WPAGE001–WPAGE002 | Error/Warning | Validates WebPage schema for name and description properties |
| 74 | Service Schema | `ServiceSchemaValidator` | SVC001–SVC002 | Error/Warning | Validates Service schema for name, provider, and description |
| 75 | ItemList Schema | `ItemListSchemaValidator` | ILIST001–ILIST002 | Error/Warning | Validates ItemList schema for itemListElement and position properties |
| 76 | Offer Schema | `OfferSchemaValidator` | OFFER001–OFFER002 | Error/Warning | Validates Offer schema for price, priceCurrency, and availability |
| 77 | AggregateOffer | `AggregateOfferSchemaValidator` | AGOFF001–AGOFF002 | Error/Warning | Validates AggregateOffer schema for lowPrice, highPrice, and priceCurrency |
| 78 | Brand Schema | `BrandSchemaValidator` | BRAND001–BRAND002 | Warning/Info | Validates Brand schema for name and url properties |
| 79 | Occupation Schema | `OccupationSchemaValidator` | OCC001–OCC002 | Error/Warning | Validates Occupation schema for name and estimatedSalary properties |
| 80 | Quest Schema | `QuestSchemaValidator` | QUEST001–QUEST002 | Warning/Info | Validates Quest schema for name and description properties |
| 81 | Action Schema | `ActionSchemaValidator` | ACTION001–ACTION002 | Warning/Info | Validates Action schema for name, target, and result properties |
| 82 | Playbook Schema | `PlaybookSchemaValidator` | PLAY001–PLAY002 | Warning/Info | Validates Playbook schema for name and step properties |
| 83 | Apartment Schema | `ApartmentSchemaValidator` | APT001–APT002 | Warning/Info | Validates Apartment schema for name, address, and floorSize |
| 84 | Car Schema | `CarSchemaValidator` | CAR001–CAR002 | Warning/Info | Validates Car schema for name, model, and vehicleConfiguration |
| 85 | Music Album | `MusicAlbumSchemaValidator` | MALB001–MALB002 | Warning/Info | Validates MusicAlbum schema for name, byArtist, and numTracks |
| 86 | TV Series | `TVSeriesSchemaValidator` | TVS001–TVS002 | Warning/Info | Validates TVSeries schema for name, season, and episode properties |
| 87 | Movie Schema | `MovieSchemaValidator` | MOV001–MOV002 | Warning/Info | Validates Movie schema for name, director, and duration |
| 88 | Government Service | `GovernmentServiceSchemaValidator` | GOV001–GOV002 | Warning/Info | Validates GovernmentService schema for name and provider |
| 89 | Health Plan | `HealthPlanSchemaValidator` | HEALTH001–HEALTH002 | Warning/Info | Validates HealthPlan schema for name and provider properties |
| 90 | Invoice Schema | `InvoiceSchemaValidator` | INV001–INV002 | Warning/Info | Validates Invoice schema for accountId, billingPeriod, and totalPaymentDue |
| 91 | Permit Schema | `PermitSchemaValidator` | PERM001–PERM002 | Warning/Info | Validates Permit schema for name and permitType properties |
| 92 | Plan Schema | `PlanSchemaValidator` | PLAN001–PLAN002 | Warning/Info | Validates Plan schema for name, price, and billingDuration |
| 93 | Product Model | `ProductModelSchemaValidator` | PMOD001–PMOD002 | Warning/Info | Validates ProductModel schema for name and manufacturer |
| 94 | Research Project | `ResearchProjectSchemaValidator` | RPROJ001–RPROJ002 | Warning/Info | Validates ResearchProject schema for name and funder |
| 95 | Schedule Schema | `ScheduleSchemaValidator` | SCHED001–SCHED002 | Warning/Info | Validates Schedule schema for byDay, startTime, and endTime |
| 96 | Trip Schema | `TripSchemaValidator` | TRIP001–TRIP002 | Warning/Info | Validates Trip schema for name and itinerary properties |
| 97 | Workers Union | `WorkersUnionSchemaValidator` | WUNION001–WUNION002 | Warning/Info | Validates WorkersUnion schema for name and member properties |
| 98 | WebAPI Schema | `WebAPISchemaValidator` | WAPI001–WAPI002 | Warning/Info | Validates WebAPI schema for name, documentation, and provider |
| 99 | Wearable Schema | `WearableSchemaValidator` | WEAR001–WEAR002 | Warning/Info | Validates Wearable schema for name and deviceType |
| 100 | WebPage Element | `WebPageElementSchemaValidator` | WPEL001–WPEL002 | Warning/Info | Validates WebPageElement schema for name and cssSelector |
| 101 | WebSite Schema | `WebSiteSchemaValidator` | WSITE001–WSITE002 | Warning/Info | Validates WebSite schema for name, url, and potentialAction |
| 102 | Worker Schema | `WorkerSchemaValidator` | WORK001–WORK002 | Warning/Info | Validates Worker schema for name and jobTitle |
| 103 | Local Business Hours | `LocalBusinessHoursValidator` | LBH001–LBH002 | Warning/Info | Validates LocalBusiness openingHours specification format |
| 104 | Product Review | `ProductReviewValidator` | PREV001–PREV002 | Warning/Info | Validates Product review schema for author, reviewRating, and body |
| 105 | Event Location | `EventLocationValidator` | ELOC001–ELOC002 | Warning/Info | Validates Event location schema for name, address, and geo |
| 106 | Organization Logo | `OrganizationLogoValidator` | OLOG001–OLOG002 | Warning/Info | Validates Organization logo schema for url, width, height, and format |
| 107 | Person Job Title | `PersonJobTitleValidator` | PJOB001–PJOB002 | Warning/Info | Validates Person jobTitle and worksFor properties |
| 108 | Recipe Nutrition | `RecipeNutritionValidator` | RNUT001–RNUT002 | Warning/Info | Validates Recipe nutrition schema for calories, protein, fat |
| 109 | Course Provider | `CourseProviderValidator` | CPROV001–CPROV002 | Warning/Info | Validates Course provider schema for name and url |
| 110 | JobPosting Salary | `JobPostingSalaryValidator` | JSAL001–JSAL002 | Warning/Info | Validates JobPosting estimatedSalary for currency and value |

### Security (24)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 111 | Security Headers | `SecurityHeaderAnalyzer` | SEC001–SEC018 | Warning/Info | Comprehensive security header audit (CSP, HSTS, XFO, XCTO, Referrer-Policy, Permissions-Policy, COEP/COOP/CORP) with posture score |
| 112 | SSL Certificate | `SslCertificateValidator` | SSL001–SSL008 | Critical/Error/Warning/Info | Validates SSL certificate expiry, chain, self-signing, hostname match, and algorithm strength |
| 113 | HSTS Preload | `HstsPreloadAnalyzer` | HSTS001–HSTS003 | Warning/Info | Validates HSTS max-age, includeSubDomains, and preload directives |
| 114 | SRI | `SriAnalyzer` | SRI001–SRI002 | Warning | Checks external scripts and stylesheets for Subresource Integrity attributes |
| 115 | Permission Policy | `PermissionPolicyAnalyzer` | PERM001–PERM002 | Warning | Validates Permissions-Policy header for camera/microphone restriction |
| 116 | Cross-Origin Isolation | `CrossOriginIsolationAnalyzer` | COEP001, COOP002 | Info | Checks for COEP and COOP headers enabling cross-origin isolation |
| 117 | Content Security Policy | `ContentSecurityPolicyAnalyzer` | CSP001–CSP002 | Warning | Detects unsafe-inline in script-src and missing frame-ancestors directive |
| 118 | Referrer Policy | `ReferrerPolicyAnalyzer` | REF001–REF002 | Warning | Validates Referrer-Policy header and flags unsafe-url configuration |
| 119 | X-Frame-Options | `XFrameOptionsAnalyzer` | XFO001–XFO002 | Warning | Validates X-Frame-Options for clickjacking protection on HTML pages |
| 120 | Mixed Content | `MixedContentAnalyzer` | MIXED001–MIXED002 | Warning/Error | Detects HTTP resources and form actions on HTTPS pages |
| 121 | Cookies | `CookieAnalyzer` | COOKIE001–COOKIE002 | Warning | Checks Set-Cookie headers for Secure and HttpOnly flags |
| 122 | X-Content-Type-Options | `XContentTypeOptionsAnalyzer` | XCTO001–XCTO002 | Warning | Validates X-Content-Type-Options: nosniff for MIME sniffing protection |
| 123 | X-Permitted-Cross-Domain | `XPermittedCrossDomainPoliciesAnalyzer` | XPCDP001–XPCDP002 | Info/Warning | Validates cross-domain policy file restrictions |
| 124 | CORP | `CrossOriginResourcePolicyAnalyzer` | CORP001 | Info | Checks Cross-Origin-Resource-Policy header for Spectre mitigation |
| 125 | Strict Transport Security | `StrictTransportSecurityAnalyzer` | STRICT001–STRICT002 | Warning | Validates HSTS header presence and max-age configuration |
| 126 | XSS Protection | `XSSProtectionAnalyzer` | XSS001–XSS002 | Info | Checks X-XSS-Protection header configuration for legacy browser support |
| 127 | Content Type Sniffing | `ContentTypeSniffingAnalyzer` | CTSNIFF001–CTSNIFF002 | Warning | Validates X-Content-Type-Options for MIME type sniffing prevention |
| 128 | Feature Policy | `FeaturePolicyAnalyzer` | FPOL001–FPOL002 | Warning/Info | Validates Feature-Policy header for browser feature restrictions |
| 129 | Expect-CT | `ExpectCTAnalyzer` | EXPECT001–EXPECT002 | Warning/Info | Validates Expect-CT header for Certificate Transparency enforcement |
| 130 | Certificate Transparency | `CertificateTransparencyAnalyzer` | CT001–CT002 | Warning/Info | Checks for CT log inclusion and sct-list header presence |

### Accessibility (21)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 131 | Accessibility | `AccessibilityAnalyzer` | A11Y001–A11Y016 | Error/Warning/Info | WCAG 2.1 AA audit: images alt, headings, landmarks, skip link, link text, forms, keyboard, tables, lang |
| 132 | Font Size | `FontSizeAnalyzer` | FSIZE001–FSIZE002 | Warning | WCAG 1.4.4/1.4.12: detects text smaller than 12px and insufficient line-height (<1.5) |
| 133 | Color Contrast | `ColorContrastAnalyzer` | CONTR001–CONTR002 | Error/Warning | WCAG 1.4.3: validates inline color contrast ratios against 3:1 and 4.5:1 thresholds |
| 134 | Focus Order | `FocusOrderAnalyzer` | FOCUS001–FOCUS002 | Error/Warning | WCAG 2.4.3: detects positive tabindex and missing focus CSS indicators |
| 135 | Landmark Regions | `LandmarkRegionsAnalyzer` | LAND001–LAND003 | Error/Warning/Info | WCAG landmark navigation: validates main, nav, and banner landmark regions |
| 136 | Heading Order | `HeadingOrderAnalyzer` | HORDER001–HORDER002 | Warning | WCAG 1.3.1: detects heading level skips and non-sequential heading order |
| 137 | Form Labels | `FormLabelAnalyzer` | FLABEL001–FLABEL002 | Error/Info | WCAG 1.3.1/4.1.2: validates form inputs have associated labels or aria-labels |
| 138 | Table Accessibility | `TableAccessibilityAnalyzer` | TACC001–TACC003 | Warning/Info | WCAG 1.3.1: validates table headers (th), captions, and scope attributes |
| 139 | Link Accessibility | `LinkAccessibilityAnalyzer` | LNKACC001–LNKACC003 | Error/Warning | WCAG 2.4.4: detects empty link text, generic text ("click here"), and non-descriptive text |
| 140 | Image Accessibility | `ImageAccessibilityAnalyzer` | IMGACC001–IMGACC003 | Error/Warning | WCAG 1.1.1: validates alt attributes, empty alt on non-decorative images, and filename-based alt |
| 141 | ARIA Roles | `AriaRolesAnalyzer` | ARIA001–ARIA002 | Warning/Info | WCAG 4.1.2: validates ARIA roles have accessible names via aria-label/aria-labelledby |
| 142 | Focus Management | `FocusManagementAnalyzer` | FOCUS001–FOCUS002 | Error/Warning | WCAG 2.4.3: validates focus order and visible focus indicators for keyboard navigation |
| 143 | Language Attribute | `LanguageAttributeAnalyzer` | LANGACC001–LANGACC002 | Error/Warning | WCAG 3.1.1: validates html lang attribute presence and hreflang consistency |

### Social (10)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 144 | Social Media | `SocialMediaAnalyzer` | SOCIAL001–SOCIAL008 | Warning/Info | Comprehensive OG/Twitter audit: image dimensions, card type, tag completeness, social score |
| 145 | OG Image Validator | `OpenGraphImageValidator` | OGIMG001–OGIMG003 | Error/Warning/Info | Validates og:image URL, dimensions (1200x630), format, and aspect ratio |
| 146 | Twitter Player | `TwitterPlayerValidator` | TWPL001–TWPL002 | Error/Warning | Validates twitter:player and twitter:player:stream for player card type |
| 147 | Social Preview Optimizer | `SocialPreviewOptimizer` | SPREV001–SPREV003 | Error/Warning | Validates og:title, og:description, and og:image URL for social preview completeness |
| 148 | OG Video | `OpenGraphVideoAnalyzer` | OGVID001–OGVID002 | Warning/Info | Validates og:video:url and og:video:type for video content sharing |
| 149 | Twitter Card Type | `TwitterCardTypeAnalyzer` | TW001–TW002 | Warning/Info | Validates twitter:card type and suggests summary_large_image when appropriate |
| 150 | OG Audio | `OpenGraphAudioAnalyzer` | OGAUDIO001–OGAUDIO002 | Warning/Info | Validates og:audio:url and og:audio:type for audio content sharing |
| 151 | Twitter Site | `TwitterSiteAnalyzer` | TWSITE001–TWSITE002 | Warning | Validates twitter:site presence and @ prefix format |

### Performance (14)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 152 | Cache Headers | `CacheHeaderAnalyzer` | CACHE001–CACHE003 | Warning/Info | Validates Cache-Control, ETag/Last-Modified, and HTML caching configuration |
| 153 | Resource Count | `ResourceCountAnalyzer` | RES001–RES002 | Warning/Info | Counts and categorizes page resources (scripts, styles, images) for load optimization |
| 154 | Critical Resource | `CriticalResourceAnalyzer` | CRIT001–CRIT003 | Warning/Info | Identifies render-blocking resources and missing preconnect hints |
| 155 | Preload Hints | `PreloadHintAnalyzer` | PRELOAD001–PRELOAD002 | Warning/Info | Validates preload hints for critical resources and flags excessive preloading |
| 156 | Async Scripts | `AsyncScriptAnalyzer` | ASYNC001–ASYNC002 | Critical/Warning | Detects render-blocking scripts without async/defer and inline script counts |
| 157 | Image Lazy Load | `ImageLazyLoadAnalyzer` | LAZYIMG001–LAZYIMG002 | Warning/Info | Validates lazy loading is applied to below-the-fold images, not above-the-fold |
| 158 | Font Display | `FontDisplayAnalyzer` | FONT001–FONT002 | Warning | Validates font-display:swap and flags excessive font file loading (>3) |
| 159 | Resource Size | `ResourceSizeAnalyzer` | RESSIZE001–RESSIZE002 | Warning/Error | Detects oversized HTML (>500KB) and estimated total page size (>5MB) |
| 160 | Connection | `ConnectionAnalyzer` | CONN001–CONN002 | Warning | Counts unique external domains and validates preconnect hints |
| 161 | Script Analyzer | `ScriptAnalyzer` | SCRIPT001–SCRIPT003 | Warning/Info | Analyzes script loading patterns, async/defer usage, and third-party scripts |
| 162 | Stylesheet Analyzer | `StylesheetAnalyzer` | STYLE001–STYLE002 | Warning/Info | Analyzes stylesheet loading patterns and render-blocking CSS |

### Images (3)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 163 | Image Analyzer | `ImageAnalyzer` | IMG001–IMG005 | Warning/Info | Validates alt text, image formats (WebP/AVIF), lazy loading, and dimensions |
| 164 | Image Aspect Ratio | `ImageAspectRatioValidator` | IAR001–IAR002 | Warning/Info | Validates image aspect ratios for layout shift prevention |
| 165 | Image File Size | `ImageFileSizeValidator` | IFS001–IFS002 | Warning/Info | Detects oversized image files that slow page loading |

### Mobile (1)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 166 | Mobile Friendliness | `MobileFriendlinessChecker` | MOB001–MOB005 | Error/Warning/Info | Validates viewport meta tag, width=device-width, user-scalable, and initial-scale |

### AI (4)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 167 | AI Crawler Accessibility | `AiCrawlerAccessibilityAnalyzer` | AI-ACC001–AI-ACC009 | Warning/Info | Checks robots.txt for AI bot blocking (GPTBot, ClaudeBot, etc.) |
| 168 | AI Content Structure | `AiContentStructureAnalyzer` | AI-CS002, AI-CS008, AI-CS009 | Warning | Validates content structure for AI extraction (subheadings, dates, author attribution) |
| 169 | AI Citation Eligibility | `AiCitationEligibilityAnalyzer` | AI-CIT001, AI-CIT005, AI-CIT007 | Info/Warning | Checks signals for AI citation (canonical, structured data, OpenGraph) |
| 170 | AI Answer Box | `AiAnswerBoxAnalyzer` | AI-AB001, AI-AB003 | Info | Detects FAQ schema opportunities and Q&A format for AI answer boxes |

### WASM (3)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 171 | WASM Pattern | `WasmPatternAnalyzer` | WASM001–WASM003 | Error/Warning | Static analysis: detects missing modulepreload, synchronous compilation, missing error handlers |
| 172 | WASM Runtime | `WasmRuntimeAnalyzer` | WASM-R001–WASM-R004 | Error/Warning | Dynamic analysis: detects WASM runtime crashes, load failures, deprecation warnings, HTTP errors |
| 173 | WASM Performance | `WasmPerformanceAnalyzer` | WASM-P001–WASM-P004 | Warning/Error | Measures WASM impact: module count, bundle size, compilation time, missing preload |

### Forms (1)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 174 | Form Analyzer | `FormAnalyzer` | FORM001–FORM002 | Warning/Info | Validates form action URLs, input types, and accessibility of form controls |

---

## Cross-Page Analyzers (19)

| # | Name | Rust Type | Finding Codes | Severity | Rationale |
|---|------|-----------|---------------|----------|-----------|
| 1 | Internal Link Graph | `InternalLinkGraphAnalyzer` | GRAPH001–GRAPH004 | Warning | Computes link graph metrics: orphan pages, link spam, navigation depth >5, authority concentration |
| 2 | Cross-Page Duplicate Content | `CrossPageDuplicateContentDetector` | DUP-CROSS001–DUP-CROSS002 | Warning | Detects near-duplicate titles and descriptions across pages using cosine similarity |
| 3 | Cannibalization Detector | `CannibalizationDetector` | CANNIB001–CANNIB002 | Warning/Error | Detects keyword cannibalization (same primary keyword) and duplicate canonical URLs |
| 4 | Orphan Page Detector | `OrphanPageDetector` | ORPHAN001 | Warning | Identifies pages with zero incoming internal links from crawled pages |
| 5 | Sitemap Coverage | `SitemapCoverageAnalyzer` | COVERAGE001 | Info | Detects crawled pages not mentioned in any sitemap |
| 6 | Link Equity Distributor | `LinkEquityDistributor` | LINK-EQ001–LINK-EQ002 | Warning | Analyzes link equity distribution: seed page dominance (>20%), unbalanced internal/external ratios |
| 7 | Redirect Chain Optimizer | `RedirectChainOptimizer` | REDIR-C001 | Warning | Detects high redirect ratios (>10% of pages are redirects) that waste crawl budget |
| 8 | Link Velocity | `LinkVelocityAnalyzer` | LINK-V001–LINK-V002 | Warning | Measures average link count per page and percentage of zero-link pages |
| 9 | Content Freshness (Cross-Page) | `ContentFreshnessCrossPageAnalyzer` | FRESH-C002 | Warning | Detects low average content depth across the crawl (>50% pages with <200 words) |
| 10 | Keyword Cannibalization | `KeywordCannibalizationAnalyzer` | KEY-CANNIB001 | Warning | Detects duplicate page titles across the crawl indicating keyword cannibalization |
| 11 | Internal Link Balance | `InternalLinkBalanceAnalyzer` | LINK-BAL001–LINK-BAL002 | Warning | Analyzes internal/external link ratio balance and dead-end page percentage |
| 12 | Crawl Quality | `CrawlQualityAnalyzer` | QUALITY001–QUALITY002 | Error/Critical | Detects high 4xx error rate (>20%) and high 5xx error rate (>10%) across crawl |
| 13 | Schema Coverage | `SchemaCoverageAnalyzer` | SCHEMA-COV001 | Warning | Detects low structured data coverage (<10% of pages have schema) |
| 14 | Mobile Readiness | `MobileReadinessAnalyzer` | MOBILE-C001 | Warning | Detects high rate of missing viewport meta tags across crawled pages |
| 15 | Security Posture | `SecurityPostureAnalyzer` | SEC-C001–SEC-C002 | Warning | Detects low CSP coverage (>30% missing) and low HSTS coverage (>50% missing) |
| 16 | Image Optimization | `ImageOptimizationAnalyzer` | IMG-OPT001 | Warning | Detects high rate of missing alt text across all crawled images |
| 17 | Heading Structure | `HeadingStructureAnalyzer` | HEAD-C001–HEAD-C002 | Warning | Detects high rate of pages without H1 (>30%) and multiple H1s (>20%) |
| 18 | Canonical Consistency | `CanonicalConsistencyAnalyzer` | CANON-C001 | Info | Reports high self-referencing canonical rate and cross-page canonical strategy |
| 19 | Overall Health Score | `OverallHealthScoreAnalyzer` | HEALTH001 | Warning | Computes crawl health score (percentage of 2xx responses) and flags scores <80 |

---

## Summary by Category

| Category | Single-Page | Cross-Page | Total |
|----------|-------------|------------|-------|
| HTTP | 7 | 1 | 8 |
| SEO | 26 | 5 | 31 |
| Content | 19 | 3 | 22 |
| Schema | 58 | 1 | 59 |
| Security | 20 | 1 | 21 |
| Accessibility | 13 | 0 | 13 |
| Social | 8 | 0 | 8 |
| Performance | 11 | 2 | 13 |
| Images | 3 | 1 | 4 |
| Mobile | 1 | 1 | 2 |
| AI | 4 | 0 | 4 |
| WASM | 3 | 0 | 3 |
| Forms | 1 | 0 | 1 |
| **Total** | **174** | **19** | **193** |

> **Source code counts:** The `AnalyzerRegistry::build_registry` function registers 180 `Box::new` entries (including 4 duplicates: `LanguageAttributeAnalyzer`, `InternalLinkAnchorAnalyzer`, `WikipediaLinkAnalyzer`, `AnchorTextDiversityAnalyzer` each appear twice). Unique single-page analyzers: **176 base + 4 AI + 3 WASM = 183**. Post-crawl: **19**. Total unique: **202**. The canonical test-asserted count is **181 + 19 = 200** (excludes conditional AI/WASM analyzers from the base count).
