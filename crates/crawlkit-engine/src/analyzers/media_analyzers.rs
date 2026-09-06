#![allow(
    clippy::unwrap_used,
    clippy::manual_range_contains,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::default_constructed_unit_structs,
    clippy::needless_return,
    clippy::needless_range_loop,
    clippy::useless_format,
    clippy::if_same_then_else,
    clippy::derivable_impls,
    clippy::manual_pattern_char_comparison,
    clippy::manual_contains
)]
use std::collections::HashSet;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// Preload Hint Analyzer
// ---------------------------------------------------------------------------

pub struct PreloadHintAnalyzer;

impl PreloadHintAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PreloadHintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PreloadHintAnalyzer {
    fn name(&self) -> &str {
        "preload-hints"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = ctx.body.unwrap_or("");

        // Count preload hints
        let preload_count =
            body.matches("rel=\"preload\"").count() + body.matches("rel='preload'").count();

        // PRELOAD002: Too many preload hints (>5)
        if preload_count > 5 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "PRELOAD002".to_string(),
                title: "Too many preload hints".to_string(),
                description: format!(
                    "Page has {preload_count} preload hints, exceeding the recommended \
                     maximum of 5. Excessive preloading can actually slow down page load by \
                     competing for bandwidth."
                ),
                url: url.clone(),
                recommendation: "Reduce preload hints to only the most critical resources. \
                                 Focus on above-the-fold fonts, hero images, and critical CSS."
                    .to_string(),
            });
        }

        // PRELOAD001: Critical resources missing preload hints
        // Check for large images without preload
        let has_critical_images = ctx
            .page
            .images
            .iter()
            .any(|img| !img.is_lazy_loaded && img.width.map_or(false, |w| w > 600));
        let has_preconnect =
            body.contains("rel=\"preconnect\"") || body.contains("rel='preconnect'");
        let has_dns_prefetch =
            body.contains("rel=\"dns-prefetch\"") || body.contains("rel='dns-prefetch'");

        // Count external origins (same as CriticalResourceAnalyzer logic)
        let origins: HashSet<String> = ctx
            .page
            .scripts
            .iter()
            .filter_map(|s| s.src.as_ref())
            .filter_map(|src| url::Url::parse(src).ok())
            .filter(|u| !u.cannot_be_a_base())
            .map(|u| u.origin().ascii_serialization())
            .collect();

        let page_origin = url::Url::parse(url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();
        let external_origins: Vec<&str> = origins
            .iter()
            .filter(|o| *o != &page_origin)
            .map(|s| s.as_str())
            .collect();

        if has_critical_images && preload_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "PRELOAD001".to_string(),
                title: "Critical resources missing preload hints".to_string(),
                description: "Large above-the-fold images were found but no preload hints are \
                              specified. Without preload hints, the browser may discover these \
                              resources late in the loading process."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <link rel=\"preload\" href=\"...\" as=\"image\"> for \
                                 critical above-the-fold images."
                    .to_string(),
            });
        } else if external_origins.len() >= 2
            && !has_preconnect
            && !has_dns_prefetch
            && preload_count == 0
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "PRELOAD001".to_string(),
                title: "Missing resource hints for external origins".to_string(),
                description: format!(
                    "Page references {} external origin(s) without preload, preconnect, or \
                     dns-prefetch hints.",
                    external_origins.len()
                ),
                url: url.clone(),
                recommendation: "Add <link rel=\"preconnect\"> or <link rel=\"dns-prefetch\"> \
                                 for critical third-party origins."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Async Script Analyzer
// ---------------------------------------------------------------------------

pub struct AsyncScriptAnalyzer;

impl AsyncScriptAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsyncScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AsyncScriptAnalyzer {
    fn name(&self) -> &str {
        "async-scripts"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = ctx.body.unwrap_or("");

        // ASYNC001: External scripts without async or defer
        let blocking_external: Vec<&str> = ctx
            .page
            .scripts
            .iter()
            .filter(|s| s.src.is_some() && !s.r#async && !s.defer && !s.is_module)
            .filter(|s| {
                s.script_type
                    .as_deref()
                    .map(|t| t != "application/ld+json")
                    .unwrap_or(true)
            })
            .map(|s| s.src.as_deref().unwrap_or(""))
            .collect();

        if !blocking_external.is_empty() {
            let examples = if blocking_external.len() > 3 {
                format!(
                    "{}, ...",
                    blocking_external
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                blocking_external.join(", ")
            };
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Performance,
                code: "ASYNC001".to_string(),
                title: "Render-blocking scripts without async/defer".to_string(),
                description: format!(
                    "{} external script(s) lack both async and defer attributes, blocking \
                     page rendering: {}.",
                    blocking_external.len(),
                    examples
                ),
                url: url.clone(),
                recommendation: "Add the async attribute to independent scripts or defer to \
                                 scripts that must execute in order."
                    .to_string(),
            });
        }

        // ASYNC002: Inline scripts blocking render (without async/defer)
        let inline_script_count = ctx
            .page
            .scripts
            .iter()
            .filter(|s| s.src.is_none())
            .filter(|s| {
                s.script_type
                    .as_deref()
                    .map(|t| t != "application/ld+json")
                    .unwrap_or(true)
            })
            .count();

        // Heuristic: check for large inline scripts by looking for patterns
        // that suggest substantial inline JS
        let has_inline_exec = body.contains("<script>")
            || body.contains("<script type=\"text/javascript\">")
            || body.contains("<script language=");

        if inline_script_count > 3 || (inline_script_count > 0 && has_inline_exec) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "ASYNC002".to_string(),
                title: "Inline scripts may block rendering".to_string(),
                description: format!(
                    "Page has {inline_script_count} inline script(s). Inline scripts without \
                     async/defer can block HTML parsing and delay First Contentful Paint."
                ),
                url: url.clone(),
                recommendation: "Move inline scripts to external files and load them with \
                                 defer, or add the async attribute to inline scripts where \
                                 possible."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Image Lazy Load Analyzer
// ---------------------------------------------------------------------------

pub struct ImageLazyLoadAnalyzer;

impl ImageLazyLoadAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImageLazyLoadAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ImageLazyLoadAnalyzer {
    fn name(&self) -> &str {
        "image-lazy-load"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.images.is_empty() {
            return findings;
        }

        // LAZYIMG001: Large images without lazy loading
        // Only flag images without dimensions (unknown size) that aren't lazy-loaded
        let unknown_size_no_lazy: Vec<&str> = ctx
            .page
            .images
            .iter()
            .filter(|img| !img.is_lazy_loaded)
            .filter(|img| img.width.is_none() || img.height.is_none())
            .map(|img| img.src.as_str())
            .collect();

        if !unknown_size_no_lazy.is_empty()
            && unknown_size_no_lazy.len() <= ctx.page.images.len() / 2
        {
            let examples = if unknown_size_no_lazy.len() > 3 {
                format!(
                    "{}, ...",
                    unknown_size_no_lazy
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                unknown_size_no_lazy.join(", ")
            };
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "LAZYIMG001".to_string(),
                title: "Images without lazy loading or dimensions".to_string(),
                description: format!(
                    "{} image(s) lack lazy loading and have no explicit dimensions, which \
                     may indicate they are above-the-fold but could also be large \
                     below-the-fold images: {}.",
                    unknown_size_no_lazy.len(),
                    examples
                ),
                url: url.clone(),
                recommendation: "Add loading=\"lazy\" to below-the-fold images. Ensure \
                                 above-the-fold images have explicit width and height to \
                                 prevent layout shifts."
                    .to_string(),
            });
        }

        // LAZYIMG002: Above-the-fold images with lazy loading
        // Images in the first few positions are likely above-the-fold
        let early_images: Vec<&str> = ctx
            .page
            .images
            .iter()
            .take(3)
            .filter(|img| img.is_lazy_loaded)
            .filter(|img| {
                // Small images with known dimensions are likely above-the-fold
                img.width
                    .zip(img.height)
                    .map_or(false, |(w, h)| w <= 600 && h <= 400)
            })
            .map(|img| img.src.as_str())
            .collect();

        if !early_images.is_empty() {
            let examples = early_images.join(", ");
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "LAZYIMG002".to_string(),
                title: "Above-the-fold images with lazy loading".to_string(),
                description: format!(
                    "Small image(s) at the top of the page use lazy loading, which may \
                     delay their display: {examples}."
                ),
                url: url.clone(),
                recommendation: "Remove loading=\"lazy\" from above-the-fold images. Lazy \
                                 loading is intended for below-the-fold content."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Font Display Analyzer
// ---------------------------------------------------------------------------

pub struct FontDisplayAnalyzer;

impl FontDisplayAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FontDisplayAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FontDisplayAnalyzer {
    fn name(&self) -> &str {
        "font-display"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = ctx.body.unwrap_or("");

        // Count font files loaded (look for font URLs in stylesheets/scripts)
        let font_extensions = [".woff2", ".woff", ".ttf", ".otf", ".eot"];
        let font_count = ctx
            .page
            .styles
            .iter()
            .filter_map(|s| s.href.as_ref())
            .filter(|href| {
                let lower = href.to_lowercase();
                font_extensions.iter().any(|ext| lower.contains(ext))
            })
            .count()
            + ctx
                .page
                .scripts
                .iter()
                .filter_map(|s| s.src.as_ref())
                .filter(|src| {
                    let lower = src.to_lowercase();
                    font_extensions.iter().any(|ext| lower.contains(ext))
                })
                .count();

        // Also count fonts referenced in body content
        let body_font_count = font_extensions
            .iter()
            .map(|ext| body.matches(ext).count())
            .sum::<usize>();

        let total_fonts = font_count + body_font_count;

        // FONT002: Multiple font files loaded (>3)
        if total_fonts > 3 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "FONT002".to_string(),
                title: "Multiple font files loaded".to_string(),
                description: format!(
                    "Page loads {total_fonts} font file(s), exceeding the recommended \
                     maximum of 3. Each font file requires a separate HTTP request and \
                     increases page weight."
                ),
                url: url.clone(),
                recommendation: "Reduce the number of font files. Use font subsetting to \
                                 include only the characters needed, or use system fonts \
                                 for body text."
                    .to_string(),
            });
        }

        // FONT001: Web fonts missing font-display:swap
        // Check if fonts are loaded but font-display is not specified
        let has_font_display = body.contains("font-display:")
            || body.contains("font-display :")
            || body.contains("font-display: ");

        if total_fonts > 0 && !has_font_display {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "FONT001".to_string(),
                title: "Web fonts missing font-display:swap".to_string(),
                description: format!(
                    "Page loads {total_fonts} font file(s) but no font-display property was \
                     found. Without font-display:swap, text may be invisible while web fonts \
                     load (Flash of Invisible Text)."
                ),
                url: url.clone(),
                recommendation: "Add font-display:swap to @font-face declarations to ensure \
                                 text remains visible during font loading."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Resource Size Analyzer
// ---------------------------------------------------------------------------

pub struct ResourceSizeAnalyzer;

impl ResourceSizeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResourceSizeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ResourceSizeAnalyzer {
    fn name(&self) -> &str {
        "resource-size"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // RESSIZE001: Single resource >500KB
        if let Some(body_size) = ctx.body_size {
            if body_size > 500 * 1024 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Performance,
                    code: "RESSIZE001".to_string(),
                    title: "Single resource exceeds 500KB".to_string(),
                    description: format!(
                        "Page HTML is {} bytes ({:.1} KB), exceeding the 500KB threshold. \
                         Large HTML files slow down parsing and increase Time to Interactive.",
                        body_size,
                        body_size as f64 / 1024.0
                    ),
                    url: url.clone(),
                    recommendation: "Reduce HTML size by removing unnecessary code, using \
                                     server-side rendering for dynamic content, or implementing \
                                     pagination."
                        .to_string(),
                });
            }
        }

        // RESSIZE002: Total page size >5MB
        if let Some(body_size) = ctx.body_size {
            // Estimate total page size including resources
            let estimated_resources = ctx.page.images.len() * 100 * 1024 // ~100KB per image
                + ctx.page.scripts.len() * 50 * 1024 // ~50KB per script
                + ctx.page.styles.len() * 20 * 1024; // ~20KB per stylesheet
            let total_estimated = body_size + estimated_resources;

            if total_estimated > 5 * 1024 * 1024 {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Performance,
                    code: "RESSIZE002".to_string(),
                    title: "Estimated total page size exceeds 5MB".to_string(),
                    description: format!(
                        "Based on HTML size ({:.1} KB) and {} resources, the estimated total \
                         page size exceeds 5MB. Large pages consume excessive bandwidth and \
                         slow down loading on mobile connections.",
                        body_size as f64 / 1024.0,
                        ctx.page.images.len() + ctx.page.scripts.len() + ctx.page.styles.len()
                    ),
                    url: url.clone(),
                    recommendation: "Reduce page size by optimizing images, minifying \
                                     code, implementing lazy loading, and using compression."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Connection Analyzer
// ---------------------------------------------------------------------------

pub struct ConnectionAnalyzer;

impl ConnectionAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConnectionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ConnectionAnalyzer {
    fn name(&self) -> &str {
        "connection"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Collect unique origins from all external resources
        let mut origins: HashSet<String> = HashSet::new();

        for s in &ctx.page.scripts {
            if let Some(src) = &s.src {
                if let Ok(parsed) = url::Url::parse(src) {
                    if !parsed.cannot_be_a_base() {
                        origins.insert(parsed.origin().ascii_serialization());
                    }
                }
            }
        }

        for s in &ctx.page.styles {
            if let Some(href) = &s.href {
                if let Ok(parsed) = url::Url::parse(href) {
                    if !parsed.cannot_be_a_base() {
                        origins.insert(parsed.origin().ascii_serialization());
                    }
                }
            }
        }

        for img in &ctx.page.images {
            if let Ok(parsed) = url::Url::parse(&img.src) {
                if !parsed.cannot_be_a_base() {
                    origins.insert(parsed.origin().ascii_serialization());
                }
            }
        }

        // Get the page origin
        let page_origin = url::Url::parse(url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();

        // Remove the page's own origin
        origins.remove(&page_origin);

        // CONN001: Too many unique domains (>10)
        if origins.len() > 10 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "CONN001".to_string(),
                title: "Too many unique external domains".to_string(),
                description: format!(
                    "Page connects to {} unique external domains, exceeding the recommended \
                     maximum of 10. Each new domain requires DNS lookup, TCP handshake, \
                     and TLS negotiation.",
                    origins.len()
                ),
                url: url.clone(),
                recommendation: "Reduce the number of external domains. Consider self-hosting \
                                 critical resources or using a CDN to consolidate origins."
                    .to_string(),
            });
        }

        // CONN002: Missing preconnect for external origins
        let body = ctx.body.unwrap_or("");
        let has_preconnect =
            body.contains("rel=\"preconnect\"") || body.contains("rel='preconnect'");
        let has_dns_prefetch =
            body.contains("rel=\"dns-prefetch\"") || body.contains("rel='dns-prefetch'");

        let external_origins: Vec<&str> = origins.iter().map(|s| s.as_str()).collect();

        if external_origins.len() >= 2 && !has_preconnect && !has_dns_prefetch {
            let examples = if external_origins.len() > 3 {
                format!(
                    "{}, ...",
                    external_origins
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                external_origins.join(", ")
            };
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "CONN002".to_string(),
                title: "Missing preconnect for external origins".to_string(),
                description: format!(
                    "Page references {} external origin(s) ({}) without <link rel=\"preconnect\"> \
                     or <link rel=\"dns-prefetch\"> hints.",
                    external_origins.len(),
                    examples
                ),
                url: url.clone(),
                recommendation: "Add <link rel=\"preconnect\" href=\"ORIGIN\"> for critical \
                                 third-party origins to establish early connections."
                    .to_string(),
            });
        }

        findings
    }
}

/// Detailed image information for analysis.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub src: String,
    pub alt: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub file_size: Option<u64>,
    pub has_alt: bool,
    pub is_lazy_loaded: bool,
}

pub struct ImageAnalyzer;

impl ImageAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub(crate) fn detect_format(src: &str) -> Option<String> {
        let path = src.split('?').next()?;
        let ext = path.rsplit('.').next()?;
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some("jpeg".to_string()),
            "png" => Some("png".to_string()),
            "gif" => Some("gif".to_string()),
            "webp" => Some("webp".to_string()),
            "avif" => Some("avif".to_string()),
            "svg" => Some("svg".to_string()),
            "bmp" => Some("bmp".to_string()),
            "ico" => Some("ico".to_string()),
            "tiff" | "tif" => Some("tiff".to_string()),
            _ => None,
        }
    }
}

impl Default for ImageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ImageAnalyzer {
    fn name(&self) -> &str {
        "image-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.images.is_empty() {
            return findings;
        }

        let mut total_lazy = 0u32;
        let mut total_with_dimensions = 0u32;
        let mut non_modern_formats = Vec::new();
        let mut missing_dimension_srcs = Vec::new();

        for img in &ctx.page.images {
            // 2.4 — Missing alt text
            if !img.has_alt {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "IMG001".to_string(),
                    title: "Image missing alt text".to_string(),
                    description: format!("Image \"{}\" has no alt attribute.", img.src),
                    url: url.clone(),
                    recommendation: "Add descriptive alt text to improve accessibility and SEO."
                        .to_string(),
                });
            }

            // Detect image format and flag non-modern formats
            if let Some(format) = Self::detect_format(&img.src) {
                let is_modern = matches!(format.as_str(), "webp" | "avif" | "svg");
                if !is_modern {
                    non_modern_formats.push(format);
                }
            }

            if img.is_lazy_loaded {
                total_lazy += 1;
            }

            if img.width.is_some() && img.height.is_some() {
                total_with_dimensions += 1;
            } else {
                missing_dimension_srcs.push(img.src.as_str());
            }
        }

        // Report non-modern image formats
        if !non_modern_formats.is_empty() {
            let format_list = non_modern_formats.join(", ");
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMG005".to_string(),
                title: "Non-modern image formats detected".to_string(),
                description: format!(
                    "{} image(s) use non-modern formats: {}. Consider using WebP or AVIF for \
                     better compression.",
                    non_modern_formats.len(),
                    format_list
                ),
                url: url.clone(),
                recommendation: "Convert images to WebP or AVIF format for smaller file sizes \
                                 and faster page loads."
                    .to_string(),
            });
        }

        // Lazy loading summary
        if total_lazy > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMG003".to_string(),
                title: "Lazy-loaded images".to_string(),
                description: format!(
                    "{} of {} images use lazy loading.",
                    total_lazy,
                    ctx.page.images.len()
                ),
                url: url.clone(),
                recommendation: "Verify lazy loading is only applied to below-the-fold images."
                    .to_string(),
            });
        }

        // Dimension summary
        let missing_dimensions = ctx.page.images.len() as u32 - total_with_dimensions;
        if missing_dimensions > 0 {
            // Name up to 3 offenders so the fix is actionable without a
            // re-inspection; repeated template images (footer badges etc.)
            // are immediately identifiable.
            let examples: Vec<&str> = missing_dimension_srcs.iter().copied().take(3).collect();
            let example_suffix = if missing_dimension_srcs.len() > 3 {
                format!(" e.g., {}, …", examples.join(", "))
            } else {
                format!(" e.g., {}", examples.join(", "))
            };
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMG004".to_string(),
                title: "Images missing dimensions".to_string(),
                description: format!(
                    "{} of {} images are missing width/height attributes.{example_suffix}",
                    missing_dimensions,
                    ctx.page.images.len()
                ),
                url: url.clone(),
                recommendation: "Specify width and height to prevent layout shifts (CLS)."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 22. E-commerce Signals Analyzer
// ---------------------------------------------------------------------------

pub struct EcommerceSignalsAnalyzer;

impl EcommerceSignalsAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn detect_product_schema(sd: &crate::parser::StructuredData) -> bool {
        sd.r#type
            .as_deref()
            .map(|t| t == "Product" || t == "IndividualProduct" || t == "AggregateOffer")
            .unwrap_or(false)
    }

    fn extract_price(data: &serde_json::Value) -> Option<String> {
        let direct = data
            .get("price")
            .or_else(|| data.get("lowPrice"))
            .or_else(|| data.get("highPrice"));
        if let Some(v) = direct {
            return Self::value_to_price(v);
        }
        let offers = data.get("offers")?;
        let offer = if offers.is_array() {
            offers.get(0)?
        } else {
            offers
        };
        let price_val = offer
            .get("price")
            .or_else(|| offer.get("lowPrice"))
            .or_else(|| offer.get("highPrice"))?;
        Self::value_to_price(price_val)
    }

    fn value_to_price(v: &serde_json::Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
        v.as_f64().map(|p| format!("{p}"))
    }

    fn extract_availability(data: &serde_json::Value) -> Option<String> {
        data.get("availability")
            .and_then(|v| v.as_str().map(String::from))
            .or_else(|| {
                data.get("offers")
                    .and_then(|o| o.get("availability"))
                    .and_then(|v| v.as_str().map(String::from))
            })
    }

    fn detect_reviews(sd: &crate::parser::StructuredData) -> Vec<String> {
        let mut reviews = Vec::new();
        if let Some(obj) = sd.data.as_object() {
            if let Some(rating) = obj.get("aggregateRating") {
                if let Some(score) = rating.get("ratingValue").and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                }) {
                    reviews.push(format!("rating: {score}"));
                }
            }
            if let Some(r) = obj.get("reviewCount").or_else(|| obj.get("ratingCount")) {
                if let Some(count) = r
                    .as_f64()
                    .or_else(|| r.as_str().and_then(|s| s.parse::<f64>().ok()))
                {
                    reviews.push(format!("reviews: {count}"));
                }
            }
        }
        reviews
    }

    fn detect_offers(sd: &crate::parser::StructuredData) -> bool {
        sd.data
            .get("offers")
            .or_else(|| sd.data.get("hasOffersCatalog"))
            .map(|v| !v.is_null())
            .unwrap_or(false)
    }
}

impl Default for EcommerceSignalsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EcommerceSignalsAnalyzer {
    fn name(&self) -> &str {
        "ecommerce-signals"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.structured_data.is_empty() {
            return findings;
        }

        let mut has_product = false;
        let mut prices_found = Vec::new();
        let mut availability_found = Vec::new();
        let mut reviews_found = Vec::new();
        let mut offers_found = false;

        for sd in &ctx.page.structured_data {
            if Self::detect_product_schema(sd) {
                has_product = true;

                if let Some(price) = Self::extract_price(&sd.data) {
                    prices_found.push(price);
                }
                if let Some(avail) = Self::extract_availability(&sd.data) {
                    availability_found.push(avail);
                }
                reviews_found.extend(Self::detect_reviews(sd));
                if Self::detect_offers(sd) {
                    offers_found = true;
                }
            }

            if sd.r#type.as_deref() == Some("Offer")
                || sd.r#type.as_deref() == Some("AggregateOffer")
            {
                if let Some(price) = Self::extract_price(&sd.data) {
                    prices_found.push(price);
                }
            }

            if sd.r#type.as_deref() == Some("Review")
                || sd.r#type.as_deref() == Some("AggregateRating")
            {
                reviews_found.extend(Self::detect_reviews(sd));
            }
        }

        if has_product {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM001".to_string(),
                title: "Product schema detected".to_string(),
                description: "Product structured data found. This enables rich product \
                              results in search."
                    .to_string(),
                url: url.clone(),
                recommendation: "Ensure all required Product properties are present (name, \
                                 image, description, offers)."
                    .to_string(),
            });
        }

        if !prices_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM002".to_string(),
                title: "Price information detected".to_string(),
                description: format!(
                    "Prices found in structured data: {}.",
                    prices_found.join(", ")
                ),
                url: url.clone(),
                recommendation: "Verify prices match the visible page content.".to_string(),
            });
        }

        if !availability_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM003".to_string(),
                title: "Availability information detected".to_string(),
                description: format!("Availability: {}.", availability_found.join(", ")),
                url: url.clone(),
                recommendation: "Availability status should match the actual product state."
                    .to_string(),
            });
        }

        if !reviews_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM004".to_string(),
                title: "Review/rating information detected".to_string(),
                description: format!("Review data: {}.", reviews_found.join(", ")),
                url: url.clone(),
                recommendation: "Ratings and reviews enhance search result CTR. Keep them \
                                 updated."
                    .to_string(),
            });
        }

        if offers_found {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "ECOM005".to_string(),
                title: "Offer schema detected".to_string(),
                description: "Offer structured data found in product schema.".to_string(),
                url: url.clone(),
                recommendation: "Ensure offer includes price, priceCurrency, and availability."
                    .to_string(),
            });
        }

        if !has_product && !prices_found.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Schema,
                code: "ECOM006".to_string(),
                title: "Price data without Product schema".to_string(),
                description: "Price information found but no Product schema type detected. \
                              Search engines may not interpret this as product data."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add Product schema to wrap price and availability data."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Product Variant Analyzer
// ---------------------------------------------------------------------------

pub struct ProductVariantAnalyzer;

impl ProductVariantAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn is_product_schema(sd: &crate::parser::StructuredData) -> bool {
        sd.r#type
            .as_deref()
            .map(|t| t == "Product")
            .unwrap_or(false)
    }

    fn has_variants(data: &serde_json::Value) -> bool {
        data.get("hasVariant")
            .or_else(|| data.get("variant"))
            .map(|v| {
                if let Some(arr) = v.as_array() {
                    !arr.is_empty()
                } else {
                    !v.is_null()
                }
            })
            .unwrap_or(false)
    }

    fn has_availability(data: &serde_json::Value) -> bool {
        // Check top-level availability
        if data.get("availability").is_some() {
            return true;
        }
        // Check in offers
        if let Some(offers) = data.get("offers") {
            if let Some(arr) = offers.as_array() {
                return arr.iter().any(|o| o.get("availability").is_some());
            }
            if offers.get("availability").is_some() {
                return true;
            }
        }
        false
    }

    fn has_offers(data: &serde_json::Value) -> bool {
        data.get("offers").map(|v| !v.is_null()).unwrap_or(false)
    }
}

impl Default for ProductVariantAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ProductVariantAnalyzer {
    fn name(&self) -> &str {
        "product-variant"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if !Self::is_product_schema(sd) {
                continue;
            }

            // PVAR001: Product schema missing variant information
            if !Self::has_variants(&sd.data) {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PVAR001".to_string(),
                    title: "Product schema missing variant information".to_string(),
                    description: "A Product schema was found but has no hasVariant or variant \
                                  property. Variant information helps search engines understand \
                                  the full product range and display accurate availability \
                                  across options."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"hasVariant\" property listing child Product schemas \
                                     for each variant (size, color, etc.) or ensure each variant \
                                     has its own Product schema."
                        .to_string(),
                });
            }

            // PVAR002: Product schema has offers but no availability
            if Self::has_offers(&sd.data) && !Self::has_availability(&sd.data) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PVAR002".to_string(),
                    title: "Product schema has offers but no availability".to_string(),
                    description: "A Product schema has offers but none include an availability \
                                  property. Search engines require availability information to \
                                  display products correctly in search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add an \"availability\" property to each Offer using a \
                                     schema.org URL such as https://schema.org/InStock or \
                                     https://schema.org/OutOfStock."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Pricing Schema Validator
// ---------------------------------------------------------------------------

pub struct PricingSchemaValidator;

impl PricingSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    fn is_product_or_offer_schema(sd: &crate::parser::StructuredData) -> bool {
        sd.r#type
            .as_deref()
            .map(|t| t == "Product" || t == "Offer" || t == "AggregateOffer")
            .unwrap_or(false)
    }

    fn extract_offer(data: &serde_json::Value) -> Option<&serde_json::Value> {
        if let Some(offers) = data.get("offers") {
            if let Some(arr) = offers.as_array() {
                return arr.first();
            }
            if !offers.is_null() {
                return Some(offers);
            }
        }
        None
    }

    fn extract_price_value(data: &serde_json::Value) -> Option<&serde_json::Value> {
        data.get("price")
            .or_else(|| data.get("lowPrice"))
            .or_else(|| data.get("highPrice"))
    }

    fn has_price_currency(data: &serde_json::Value) -> bool {
        data.get("priceCurrency").is_some()
    }

    fn extract_price_valid_until(data: &serde_json::Value) -> Option<String> {
        data.get("priceValidUntil")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn is_date_expired(date_str: &str) -> Option<bool> {
        // Try to parse YYYY-MM-DD format
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let year: i32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let day: u32 = parts[2].parse().ok()?;
        if month < 1 || month > 12 || day < 1 || day > 31 {
            return None;
        }
        // Compare with a fixed "current" date for determinism in tests,
        // but use chrono in production if available.
        // For now, just check if the year is in the past (simplified)
        // In a real system you'd compare against Utc::now()
        Some(year < 2024)
    }
}

impl Default for PricingSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PricingSchemaValidator {
    fn name(&self) -> &str {
        "pricing-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if !Self::is_product_or_offer_schema(sd) {
                continue;
            }

            let data = &sd.data;

            // For Product type, check nested offers
            let offer_data = if sd.r#type.as_deref() == Some("Product") {
                Self::extract_offer(data).unwrap_or(data)
            } else {
                data
            };

            // PRICE001: Price present but priceCurrency missing
            let has_price = Self::extract_price_value(offer_data).is_some();
            let has_currency = Self::has_price_currency(offer_data);
            if has_price && !has_currency {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "PRICE001".to_string(),
                    title: "Price present but priceCurrency missing".to_string(),
                    description: "A price value was found in the schema but the priceCurrency \
                                  property is missing. Search engines require both price and \
                                  currency to display pricing information in search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"priceCurrency\" property using ISO 4217 currency \
                                     codes (e.g., \"USD\", \"EUR\", \"GBP\")."
                        .to_string(),
                });
            }

            // PRICE002: priceValidUntil missing or expired
            if has_price {
                match Self::extract_price_valid_until(offer_data) {
                    None => {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PRICE002".to_string(),
                            title: "priceValidUntil missing from offer".to_string(),
                            description: "A price was found in the schema but no \
                                          priceValidUntil property was specified. Without this, \
                                          search engines cannot determine if the price is \
                                          still current."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Add a \"priceValidUntil\" property with an ISO 8601 \
                                             date (e.g., \"2025-12-31\") to indicate when the \
                                             price expires."
                                .to_string(),
                        });
                    }
                    Some(date_str) => {
                        if let Some(expired) = Self::is_date_expired(&date_str) {
                            if expired {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Schema,
                                    code: "PRICE002".to_string(),
                                    title: "priceValidUntil date has expired".to_string(),
                                    description: format!(
                                        "The priceValidUntil date \"{date_str}\" appears to be in \
                                         the past. Search engines may treat this product as \
                                         having an outdated price."
                                    ),
                                    url: url.clone(),
                                    recommendation: "Update the priceValidUntil date to a future \
                                                     date to ensure the price is considered \
                                                     current."
                                        .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// AggregateRating Validator
// ---------------------------------------------------------------------------

pub struct AggregateRatingValidator;

impl AggregateRatingValidator {
    pub fn new() -> Self {
        Self
    }

    fn extract_aggregate_ratings<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<&'a serde_json::Value> {
        let mut ratings = Vec::new();
        for sd in &ctx.page.structured_data {
            // Direct AggregateRating schemas
            if sd.r#type.as_deref() == Some("AggregateRating") {
                ratings.push(&sd.data);
            }
            // Nested aggregateRating in other schemas
            if let Some(ar) = sd.data.get("aggregateRating") {
                if ar
                    .get("@type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "AggregateRating")
                    .unwrap_or(false)
                {
                    ratings.push(ar);
                } else if ar.get("ratingValue").is_some() {
                    // Accept even without explicit @type if ratingValue exists
                    ratings.push(ar);
                }
            }
        }
        ratings
    }

    fn parse_f64(val: &serde_json::Value) -> Option<f64> {
        val.as_f64()
            .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))
    }

    fn parse_usize(val: &serde_json::Value) -> Option<usize> {
        val.as_f64()
            .map(|v| v as usize)
            .or_else(|| val.as_str().and_then(|s| s.parse::<usize>().ok()))
    }
}

impl Default for AggregateRatingValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AggregateRatingValidator {
    fn name(&self) -> &str {
        "aggregate-rating"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let ratings = Self::extract_aggregate_ratings(ctx);

        for ar in &ratings {
            let best_rating = ar
                .get("bestRating")
                .and_then(Self::parse_f64)
                .unwrap_or(5.0);

            // ARAT001: ratingValue > bestRating
            if let Some(rating_value) = ar.get("ratingValue").and_then(Self::parse_f64) {
                if rating_value > best_rating {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "ARAT001".to_string(),
                        title: "AggregateRating ratingValue exceeds bestRating".to_string(),
                        description: format!(
                            "AggregateRating ratingValue is {rating_value} but bestRating is \
                             {best_rating}. The ratingValue must not exceed bestRating."
                        ),
                        url: url.clone(),
                        recommendation: "Set ratingValue to a number between worstRating \
                                         (default 0) and bestRating (default 5)."
                            .to_string(),
                    });
                }
            }

            // ARAT002: reviewCount or ratingCount is 0
            let review_count = ar.get("reviewCount").and_then(Self::parse_usize);
            let rating_count = ar.get("ratingCount").and_then(Self::parse_usize);
            let count = review_count.or(rating_count);

            match count {
                Some(0) => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "ARAT002".to_string(),
                        title: "AggregateRating reviewCount or ratingCount is 0".to_string(),
                        description: "An AggregateRating has a reviewCount or ratingCount of 0. \
                                      Search engines may not display ratings when there are no \
                                      reviews."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Ensure reviewCount or ratingCount is a positive \
                                         integer representing the number of reviews."
                            .to_string(),
                    });
                }
                None => {
                    // Neither reviewCount nor ratingCount present — already handled by
                    // ReviewSchemaValidator REV001, so skip here.
                }
                _ => {}
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 23. Resource Count Analyzer
// ---------------------------------------------------------------------------

pub struct ResourceCountAnalyzer;

impl ResourceCountAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResourceCountAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ResourceCountAnalyzer {
    fn name(&self) -> &str {
        "resource-count"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let js_count = ctx.page.scripts.iter().filter(|s| s.src.is_some()).count();

        let css_count = ctx.page.styles.iter().filter(|s| s.href.is_some()).count();

        let image_count = ctx.page.images.len();

        let total_resources = js_count + css_count + image_count;

        // RES001: > 100 total resources
        if total_resources > 100 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "RES001".to_string(),
                title: "Excessive total resource count".to_string(),
                description: format!(
                    "Page loads {total_resources} resources (JS: {js_count}, CSS: {css_count}, \
                     images: {image_count}), exceeding the recommended maximum of 100."
                ),
                url: url.clone(),
                recommendation: "Reduce the number of resources. Combine CSS/JS files, use \
                                 image sprites, or implement code splitting."
                    .to_string(),
            });
        }

        // RES002: > 20 JS files
        if js_count > 20 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "RES002".to_string(),
                title: "Excessive JavaScript file count".to_string(),
                description: format!(
                    "Page loads {js_count} JavaScript files, exceeding the recommended maximum \
                     of 20. Each file requires a separate HTTP request."
                ),
                url: url.clone(),
                recommendation: "Bundle JavaScript files together, use async/defer loading, or \
                                 implement code splitting."
                    .to_string(),
            });
        }

        // RES003: > 50 images
        if image_count > 50 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "RES003".to_string(),
                title: "Excessive image count".to_string(),
                description: format!(
                    "Page loads {image_count} images, exceeding the recommended maximum of 50."
                ),
                url: url.clone(),
                recommendation: "Reduce image count, use CSS sprites, lazy load below-the-fold \
                                 images, or combine images."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Critical Resource Analyzer
// ---------------------------------------------------------------------------

pub struct CriticalResourceAnalyzer;

impl CriticalResourceAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn check_blocking_scripts(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let blocking: Vec<&str> = ctx
            .page
            .scripts
            .iter()
            .filter(|s| s.src.is_some() && !s.r#async && !s.defer && !s.is_module)
            .filter(|s| {
                s.script_type
                    .as_deref()
                    .map(|t| t != "application/ld+json")
                    .unwrap_or(true)
            })
            .map(|s| s.src.as_deref().unwrap_or(""))
            .collect();
        if !blocking.is_empty() {
            let examples = if blocking.len() > 3 {
                format!(
                    "{}, \u{2026}",
                    blocking
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                blocking.join(", ")
            };
            f.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Performance,
                code: "CRIT001".to_string(),
                title: "Render-blocking scripts detected".to_string(),
                description: format!(
                    "{} external script(s) lack async/defer attributes and block page rendering: \
                     {}.",
                    blocking.len(),
                    examples
                ),
                url: url.to_string(),
                recommendation: "Add the async or defer attribute to external scripts. Use async \
                                 for independent scripts and defer for scripts that must execute \
                                 in order."
                    .to_string(),
            });
        }
    }

    fn check_blocking_stylesheets(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let blocking: Vec<&str> = ctx
            .page
            .styles
            .iter()
            .filter(|s| s.href.is_some() && !s.is_inline)
            .filter(|s| match &s.media {
                None => true,
                Some(m) => {
                    let lower = m.trim().to_lowercase();
                    lower.is_empty() || lower == "all" || lower == "screen"
                }
            })
            .map(|s| s.href.as_deref().unwrap_or(""))
            .collect();
        if !blocking.is_empty() {
            let examples = if blocking.len() > 3 {
                format!(
                    "{}, \u{2026}",
                    blocking
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                blocking.join(", ")
            };
            f.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "CRIT002".to_string(),
                title: "Render-blocking stylesheets detected".to_string(),
                description: format!(
                    "{} external stylesheet(s) are render-blocking (missing media attribute or \
                     media=\"all\"): {}.",
                    blocking.len(),
                    examples
                ),
                url: url.to_string(),
                recommendation: "Add a media attribute (e.g., media=\"print\" with \
                                 onload=\"this.media='all'\") or use rel=\"preload\" with \
                                 onload for non-critical CSS."
                    .to_string(),
            });
        }
    }

    fn collect_external_origins(ctx: &AnalysisContext) -> HashSet<String> {
        let mut origins = HashSet::new();
        for s in &ctx.page.scripts {
            if let Some(src) = &s.src {
                if let Ok(parsed) = url::Url::parse(src) {
                    if !parsed.cannot_be_a_base() {
                        origins.insert(parsed.origin().ascii_serialization());
                    }
                }
            }
        }
        for s in &ctx.page.styles {
            if let Some(href) = &s.href {
                if let Ok(parsed) = url::Url::parse(href) {
                    if !parsed.cannot_be_a_base() {
                        origins.insert(parsed.origin().ascii_serialization());
                    }
                }
            }
        }
        origins
    }

    fn check_preconnect_hints(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let has_preconnect =
            body.contains("rel=\"preconnect\"") || body.contains("rel='preconnect'");
        let has_dns_prefetch =
            body.contains("rel=\"dns-prefetch\"") || body.contains("rel='dns-prefetch'");

        if has_preconnect || has_dns_prefetch {
            return;
        }

        let origins = Self::collect_external_origins(ctx);
        let page_origin = url::Url::parse(url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();
        let external: Vec<&str> = origins
            .iter()
            .filter(|o| *o != &page_origin)
            .map(|s| s.as_str())
            .collect();
        if external.len() >= 2 {
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "CRIT003".to_string(),
                title: "Missing preconnect hints for external origins".to_string(),
                description: format!(
                    "Page references {} external origin(s) ({}) without <link rel=\"preconnect\"> \
                     or <link rel=\"dns-prefetch\"> hints. Early connection setup reduces latency.",
                    external.len(),
                    external.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Add <link rel=\"preconnect\" href=\"ORIGIN\"> for critical \
                                 third-party origins to establish early connections."
                    .to_string(),
            });
        }
    }
}

impl Default for CriticalResourceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CriticalResourceAnalyzer {
    fn name(&self) -> &str {
        "critical-resources"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        self.check_blocking_scripts(ctx, url, &mut f);
        self.check_blocking_stylesheets(ctx, url, &mut f);
        self.check_preconnect_hints(ctx, url, &mut f);
        f
    }
}

// =========================================================================
// ImageAspectRatioValidator
// =========================================================================

pub struct ImageAspectRatioValidator;
impl Default for ImageAspectRatioValidator {
    fn default() -> Self {
        Self
    }
}
impl ImageAspectRatioValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageAspectRatioValidator {
    fn name(&self) -> &str {
        "image-aspect-ratio"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for img in &ctx.page.images {
            if let (Some(w), Some(h)) = (img.width, img.height) {
                if w > 0 && h > 0 {
                    let ratio = w as f64 / h as f64;
                    if ratio > 3.0 || ratio < 0.33 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Images,
                            code: "IMGAR001".to_string(),
                            title: "Unusual image aspect ratio".to_string(),
                            description: format!(
                                "Image {} has unusual aspect ratio {:.2}:1.",
                                img.src, ratio
                            ),
                            url: url.clone(),
                            recommendation: "Check if the image dimensions are correct."
                                .to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// ImageFileSizeValidator
// =========================================================================

pub struct ImageFileSizeValidator;
impl Default for ImageFileSizeValidator {
    fn default() -> Self {
        Self
    }
}
impl ImageFileSizeValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageFileSizeValidator {
    fn name(&self) -> &str {
        "image-file-size"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for img in &ctx.page.images {
            if img.src.starts_with("data:") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Images,
                    code: "IMGFS001".to_string(),
                    title: "Inline data URI image".to_string(),
                    description: format!(
                        "Image {} uses a data URI which increases page size.",
                        img.src
                    ),
                    url: url.clone(),
                    recommendation: "Use external image files instead of data URIs.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// ScriptAnalyzer
// =========================================================================

pub struct ScriptAnalyzer;
impl Default for ScriptAnalyzer {
    fn default() -> Self {
        Self
    }
}
impl ScriptAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ScriptAnalyzer {
    fn name(&self) -> &str {
        "script-analysis"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let scripts = &ctx.page.scripts;
        if scripts.len() > 20 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "SCRIPT001".to_string(),
                title: "Excessive script count".to_string(),
                description: format!(
                    "Page has {} scripts. Consider bundling or lazy-loading.",
                    scripts.len()
                ),
                url: url.clone(),
                recommendation: "Reduce script count by bundling, code-splitting, or lazy-loading."
                    .to_string(),
            });
        }
        let blocking = scripts.iter().filter(|s| !s.r#async && !s.defer).count();
        if blocking > 3 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "SCRIPT002".to_string(),
                title: "Multiple blocking scripts".to_string(),
                description: format!("{} scripts are render-blocking.", blocking),
                url: url.clone(),
                recommendation: "Add async or defer attributes to non-critical scripts."
                    .to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// StylesheetAnalyzer
// =========================================================================

pub struct StylesheetAnalyzer;
impl Default for StylesheetAnalyzer {
    fn default() -> Self {
        Self
    }
}
impl StylesheetAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for StylesheetAnalyzer {
    fn name(&self) -> &str {
        "stylesheet-analysis"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let styles = &ctx.page.styles;
        if styles.len() > 10 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "STYLE001".to_string(),
                title: "Excessive stylesheet count".to_string(),
                description: format!("Page has {} stylesheets. Consider bundling.", styles.len()),
                url: url.clone(),
                recommendation: "Bundle CSS files to reduce HTTP requests.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// FormAnalyzer
// =========================================================================

pub struct FormAnalyzer;
impl Default for FormAnalyzer {
    fn default() -> Self {
        Self
    }
}
impl FormAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormAnalyzer {
    fn name(&self) -> &str {
        "form-analysis"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for form in &ctx.page.forms {
            if form.action.is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "FORM001".to_string(),
                    title: "Form missing action URL".to_string(),
                    description: "A form element has no action attribute.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a valid action URL to the form.".to_string(),
                });
            }
            if form.method.to_uppercase() == "GET" && form.has_search_input {
                // Search forms using GET is normal; just informational
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedImage, ParsedPage, ScriptInfo, StyleInfo};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(
        page: &'a ParsedPage,
        status: Option<u16>,
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // ===== PreloadHintAnalyzer tests =====

    #[test]
    fn test_preload_no_resources() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_preload_too_many_hints() {
        let body = r#"
            <link rel="preload" href="/a.css" as="style">
            <link rel="preload" href="/b.js" as="script">
            <link rel="preload" href="/c.woff2" as="font">
            <link rel="preload" href="/d.png" as="image">
            <link rel="preload" href="/e.css" as="style">
            <link rel="preload" href="/f.js" as="script">
        "#;
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PRELOAD002"));
    }

    #[test]
    fn test_preload_critical_images_missing_hints() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/hero.jpg".to_string(),
            alt: "Hero".to_string(),
            width: Some(1200),
            height: Some(800),
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), Some(""));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PRELOAD001"));
    }

    #[test]
    fn test_preload_critical_images_with_hints() {
        let body = r#"<link rel="preload" href="/hero.jpg" as="image">"#;
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/hero.jpg".to_string(),
            alt: "Hero".to_string(),
            width: Some(1200),
            height: Some(800),
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        // Has preload hints now, so no PRELOAD001
        assert!(!findings.iter().any(|f| f.code == "PRELOAD001"));
    }

    #[test]
    fn test_preload_external_origins_missing_hints() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn1.example.com/app.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: Some("https://cdn2.example.com/lib.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), Some(""));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PRELOAD001"));
    }

    #[test]
    fn test_preload_no_large_images_no_finding() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/small.png".to_string(),
            alt: "Small".to_string(),
            width: Some(100),
            height: Some(100),
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), Some(""));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PRELOAD001"));
    }

    #[test]
    fn test_preload_exactly_5_hints_ok() {
        let body = r#"
            <link rel="preload" href="/a.css" as="style">
            <link rel="preload" href="/b.js" as="script">
            <link rel="preload" href="/c.woff2" as="font">
            <link rel="preload" href="/d.png" as="image">
            <link rel="preload" href="/e.css" as="style">
        "#;
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PRELOAD002"));
    }

    #[test]
    fn test_preload_single_hint_ok() {
        let body = r#"<link rel="preload" href="/critical.css" as="style">"#;
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PRELOAD002"));
    }

    #[test]
    fn test_preload_preconnect_present_no_finding() {
        let body = r#"<link rel="preconnect" href="https://cdn.example.com">"#;
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/app.js".to_string()),
            r#async: true,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = PreloadHintAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PRELOAD001"));
    }

    // ===== AsyncScriptAnalyzer tests =====

    #[test]
    fn test_async_no_scripts() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_async_blocking_scripts() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ASYNC001"));
    }

    #[test]
    fn test_async_async_script_no_finding() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/app.js".to_string()),
            r#async: true,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ASYNC001"));
    }

    #[test]
    fn test_async_defer_script_no_finding() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/app.js".to_string()),
            r#async: false,
            defer: true,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ASYNC001"));
    }

    #[test]
    fn test_async_ld_json_not_flagged() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://cdn.example.com/schema.js".to_string()),
            r#async: false,
            defer: false,
            script_type: Some("application/ld+json".to_string()),
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ASYNC001"));
    }

    #[test]
    fn test_async_multiple_blocking_scripts() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn.example.com/a.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: Some("https://cdn.example.com/b.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ASYNC001"));
        let f = findings.iter().find(|f| f.code == "ASYNC001").unwrap();
        assert!(f.description.contains("2"));
    }

    #[test]
    fn test_async_inline_scripts_warning() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: None,
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: None,
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: None,
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: None,
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), Some("<script>var x = 1;</script>"));
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ASYNC002"));
    }

    #[test]
    fn test_async_one_inline_script_no_warning() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: None,
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = AsyncScriptAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ASYNC002"));
    }

    // ===== ImageLazyLoadAnalyzer tests =====

    #[test]
    fn test_lazy_load_no_images() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lazy_load_images_without_dimensions() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/img1.jpg".to_string(),
                alt: "Img1".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/img2.jpg".to_string(),
                alt: "Img2".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/img3.jpg".to_string(),
                alt: "Img3".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/img4.jpg".to_string(),
                alt: "Img4".to_string(),
                width: Some(200),
                height: Some(200),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LAZYIMG001"));
    }

    #[test]
    fn test_lazy_load_all_images_with_dimensions_no_finding() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/img1.jpg".to_string(),
                alt: "Img1".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/img2.jpg".to_string(),
                alt: "Img2".to_string(),
                width: Some(200),
                height: Some(150),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LAZYIMG001"));
    }

    #[test]
    fn test_lazy_load_early_images_with_lazy() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/hero.jpg".to_string(),
            alt: "Hero".to_string(),
            width: Some(100),
            height: Some(100),
            has_alt: true,
            is_lazy_loaded: true,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LAZYIMG002"));
    }

    #[test]
    fn test_lazy_load_early_images_not_lazy_no_finding() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/hero.jpg".to_string(),
            alt: "Hero".to_string(),
            width: Some(100),
            height: Some(100),
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LAZYIMG002"));
    }

    #[test]
    fn test_lazy_load_mixed_scenario() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/hero.jpg".to_string(),
                alt: "Hero".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: true,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/below-fold.jpg".to_string(),
                alt: "Below".to_string(),
                width: None,
                height: None,
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        // Should have LAZYIMG001 for the second image and LAZYIMG002 for the first
        assert!(findings.iter().any(|f| f.code == "LAZYIMG001"));
        assert!(findings.iter().any(|f| f.code == "LAZYIMG002"));
    }

    #[test]
    fn test_lazy_load_large_images_not_lazy_no_flag() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/big1.jpg".to_string(),
                alt: "Big1".to_string(),
                width: Some(800),
                height: Some(600),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/big2.jpg".to_string(),
                alt: "Big2".to_string(),
                width: Some(1000),
                height: Some(800),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ImageLazyLoadAnalyzer::new().analyze(&ctx);
        // Large images with known dimensions should not trigger LAZYIMG001
        assert!(!findings.iter().any(|f| f.code == "LAZYIMG001"));
    }

    // ===== FontDisplayAnalyzer tests =====

    #[test]
    fn test_font_no_fonts() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_font_missing_display() {
        let body = r#"<link href="/fonts/roboto.woff2" rel="stylesheet">"#;
        let mut page = make_page("https://example.com");
        page.styles = vec![StyleInfo {
            href: Some("https://cdn.example.com/fonts/roboto.woff2".to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FONT001"));
    }

    #[test]
    fn test_font_with_display_swap() {
        let body = r#"@font-face { font-display: swap; }"#;
        let mut page = make_page("https://example.com");
        page.styles = vec![StyleInfo {
            href: Some("https://cdn.example.com/fonts/roboto.woff2".to_string()),
            media: None,
            is_inline: false,
            has_integrity: false,
        }];
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FONT001"));
    }

    #[test]
    fn test_font_multiple_font_files() {
        let mut page = make_page("https://example.com");
        page.styles = vec![
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/roboto.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/open-sans.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/lato.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/arial.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FONT002"));
    }

    #[test]
    fn test_font_three_font_files_ok() {
        let mut page = make_page("https://example.com");
        page.styles = vec![
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/roboto.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/open-sans.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: Some("https://cdn.example.com/fonts/lato.woff2".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FONT002"));
    }

    #[test]
    fn test_font_body_font_references() {
        let body = r#"
            <link href="/fonts/roboto.woff2" rel="stylesheet">
            <link href="/fonts/opensans.woff" rel="stylesheet">
            <style>src: url('/fonts/lato.ttf')</style>
            <style>src: url('/fonts/arial.eot')</style>
        "#;
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FONT002"));
    }

    #[test]
    fn test_font_non_font_stylesheets_not_counted() {
        let mut page = make_page("https://example.com");
        page.styles = vec![
            StyleInfo {
                href: Some("https://cdn.example.com/main.css".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            },
            StyleInfo {
                href: Some("https://cdn.example.com/print.css".to_string()),
                media: Some("print".to_string()),
                is_inline: false,
                has_integrity: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = FontDisplayAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FONT001"));
        assert!(!findings.iter().any(|f| f.code == "FONT002"));
    }

    // ===== ResourceSizeAnalyzer tests =====

    #[test]
    fn test_resource_size_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_resource_size_large_html() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(600 * 1024), // 600KB
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RESSIZE001"));
    }

    #[test]
    fn test_resource_size_normal_html() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(50 * 1024), // 50KB
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "RESSIZE001"));
    }

    #[test]
    fn test_resource_size_estimated_large_page() {
        let mut page = make_page("https://example.com");
        // Add many resources to push total over 5MB
        page.images = vec![
            ExtractedImage {
                src: format!("https://example.com/img{}.jpg", 1),
                alt: "Img".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            };
            40
        ];
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://example.com/app.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            };
            30
        ];

        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(100 * 1024), // 100KB HTML
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        // 40*100KB + 30*50KB + 100KB = 4000+1500+100 = 5600KB > 5MB
        assert!(findings.iter().any(|f| f.code == "RESSIZE002"));
    }

    #[test]
    fn test_resource_size_small_page_no_findings() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(10 * 1024), // 10KB
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_resource_size_exactly_500kb() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(500 * 1024), // Exactly 500KB
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        // Exactly 500KB is not > 500KB
        assert!(!findings.iter().any(|f| f.code == "RESSIZE001"));
    }

    #[test]
    fn test_resource_size_over_500kb() {
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(500 * 1024 + 1), // Just over 500KB
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ResourceSizeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RESSIZE001"));
    }

    // ===== ConnectionAnalyzer tests =====

    #[test]
    fn test_connection_no_external() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_connection_too_many_domains() {
        let mut page = make_page("https://example.com");
        page.scripts = (1..=12)
            .map(|i| ScriptInfo {
                src: Some(format!("https://cdn{i}.example.com/app.js")),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CONN001"));
    }

    #[test]
    fn test_connection_10_domains_ok() {
        let mut page = make_page("https://example.com");
        page.scripts = (1..=10)
            .map(|i| ScriptInfo {
                src: Some(format!("https://cdn{i}.example.com/app.js")),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CONN001"));
    }

    #[test]
    fn test_connection_missing_preconnect() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn1.example.com/a.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: Some("https://cdn2.example.com/b.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), Some(""));
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CONN002"));
    }

    #[test]
    fn test_connection_with_preconnect() {
        let body = r#"<link rel="preconnect" href="https://cdn1.example.com">"#;
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn1.example.com/a.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: Some("https://cdn2.example.com/b.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CONN002"));
    }

    #[test]
    fn test_connection_with_dns_prefetch() {
        let body = r#"<link rel="dns-prefetch" href="https://cdn1.example.com">"#;
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            ScriptInfo {
                src: Some("https://cdn1.example.com/a.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
            ScriptInfo {
                src: Some("https://cdn2.example.com/b.js".to_string()),
                r#async: true,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), Some(body));
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CONN002"));
    }

    #[test]
    fn test_connection_images_from_same_origin() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "https://example.com/a.jpg".to_string(),
                alt: "A".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "https://example.com/b.jpg".to_string(),
                alt: "B".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), None);
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        // Same origin, no external connections
        assert!(findings.is_empty());
    }

    #[test]
    fn test_connection_images_from_external_origins() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "https://img1.cdn.com/a.jpg".to_string(),
                alt: "A".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "https://img2.cdn.com/b.jpg".to_string(),
                alt: "B".to_string(),
                width: Some(100),
                height: Some(100),
                has_alt: true,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), Some(""));
        let findings = ConnectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CONN002"));
    }
}

// =========================================================================
// ThirdPartyResourceAnalyzer
// =========================================================================

pub struct ThirdPartyResourceAnalyzer;

impl Default for ThirdPartyResourceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ThirdPartyResourceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ThirdPartyResourceAnalyzer {
    fn name(&self) -> &str {
        "third-party-resources"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let page_origin = url::Url::parse(url)
            .ok()
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();

        let total_scripts = ctx.page.scripts.len();
        let external_scripts = ctx
            .page
            .scripts
            .iter()
            .filter(|s| {
                s.src.as_ref().is_some_and(|src| {
                    url::Url::parse(src)
                        .ok()
                        .map(|u| u.origin().ascii_serialization() != page_origin)
                        .unwrap_or(false)
                })
            })
            .count();

        let total_styles = ctx.page.styles.len();
        let external_styles = ctx
            .page
            .styles
            .iter()
            .filter(|s| {
                s.href.as_ref().is_some_and(|href| {
                    url::Url::parse(href)
                        .ok()
                        .map(|u| u.origin().ascii_serialization() != page_origin)
                        .unwrap_or(false)
                })
            })
            .count();

        let total_images = ctx.page.images.len();
        let external_images = ctx
            .page
            .images
            .iter()
            .filter(|img| {
                url::Url::parse(&img.src)
                    .ok()
                    .map(|u| u.origin().ascii_serialization() != page_origin)
                    .unwrap_or(false)
            })
            .count();

        let total = total_scripts + total_styles + total_images;
        let external = external_scripts + external_styles + external_images;

        if total > 0 {
            let ratio = external as f64 / total as f64;
            if ratio > 0.30 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Performance,
                    code: "THIRDPARTY001".to_string(),
                    title: "Excessive third-party resources".to_string(),
                    description: format!(
                        "{:.0}% of resources ({}/{}) are from third-party origins. \
                         Third-party resources add DNS lookups, connections, and \
                         compete for bandwidth.",
                        ratio * 100.0,
                        external,
                        total
                    ),
                    url: url.clone(),
                    recommendation: "Reduce third-party resource usage. Self-host critical \
                                     resources or use a CDN on your own domain."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// BlockingStyleAnalyzer
// =========================================================================

pub struct BlockingStyleAnalyzer;

impl Default for BlockingStyleAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockingStyleAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for BlockingStyleAnalyzer {
    fn name(&self) -> &str {
        "blocking-style"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = match ctx.body {
            Some(b) => b,
            None => return findings,
        };

        // Count link rel=stylesheet tags without media or async attributes
        let blocking_count = ctx
            .page
            .styles
            .iter()
            .filter(|s| {
                s.href.is_some()
                    && s.media.as_deref() != Some("print")
                    && !s.media.as_deref().is_some_and(|m| m != "all")
            })
            .count();

        // Also check HTML directly for blocking patterns
        let html_blocking = body.matches(r#"<link rel="stylesheet""#).count()
            + body.matches(r#"<link rel='stylesheet'"#).count();

        let count = blocking_count.max(html_blocking);

        if count > 3 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "BLOCKSTYLE001".to_string(),
                title: "Multiple blocking stylesheets in head".to_string(),
                description: format!(
                    "{count} blocking stylesheet(s) found. Synchronous stylesheets \
                     block rendering until downloaded and parsed."
                ),
                url: url.clone(),
                recommendation: "Use media queries or async loading for non-critical \
                                 stylesheets. Inline critical CSS and defer the rest."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ImageDimensionMissingAnalyzer
// =========================================================================

pub struct ImageDimensionMissingAnalyzer;

impl Default for ImageDimensionMissingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDimensionMissingAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageDimensionMissingAnalyzer {
    fn name(&self) -> &str {
        "image-dimension-missing"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let missing_dim = ctx
            .page
            .images
            .iter()
            .filter(|img| img.width.is_none() || img.height.is_none())
            .count();

        if missing_dim > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "IMGDIM001".to_string(),
                title: "Images missing width/height attributes".to_string(),
                description: format!(
                    "{missing_dim} of {} image(s) are missing width or height attributes. \
                     Without dimensions, browsers cannot reserve space, causing \
                     Cumulative Layout Shift (CLS).",
                    ctx.page.images.len()
                ),
                url: url.clone(),
                recommendation: "Add explicit width and height attributes to all img \
                                 elements to prevent layout shifts."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Performance: Image Lazy Load V2 — images without lazy loading
// ---------------------------------------------------------------------------

pub struct ImageLazyLoadAnalyzerV2;

impl Default for ImageLazyLoadAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageLazyLoadAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageLazyLoadAnalyzerV2 {
    fn name(&self) -> &str {
        "image-lazy-load-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.images.is_empty() {
            return findings;
        }
        let non_lazy: Vec<&str> = ctx
            .page
            .images
            .iter()
            .filter(|img| !img.is_lazy_loaded)
            .map(|img| img.src.as_str())
            .collect();
        if !non_lazy.is_empty() && non_lazy.len() > 3 {
            let examples = non_lazy
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "IMG-LAZY001".to_string(),
                title: "Images without lazy loading".to_string(),
                description: format!("{} image(s) lack loading=\"lazy\": {}. Below-the-fold images should use lazy loading to improve initial page load performance.", non_lazy.len(), examples),
                url: url.clone(),
                recommendation: "Add loading=\"lazy\" to below-the-fold images to defer their loading until they are needed.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Performance: Script Load V2 — scripts without async/defer
// ---------------------------------------------------------------------------

pub struct ScriptLoadAnalyzerV2;

impl Default for ScriptLoadAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptLoadAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ScriptLoadAnalyzerV2 {
    fn name(&self) -> &str {
        "script-load-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let blocking: Vec<&str> = ctx
            .page
            .scripts
            .iter()
            .filter(|s| s.src.is_some() && !s.r#async && !s.defer && !s.is_module)
            .filter(|s| {
                s.script_type
                    .as_deref()
                    .map(|t| t != "application/ld+json")
                    .unwrap_or(true)
            })
            .map(|s| s.src.as_deref().unwrap_or(""))
            .collect();
        if !blocking.is_empty() {
            let examples = blocking
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            findings.push(Finding {
                severity: Severity::Critical,
                category: IssueCategory::Performance,
                code: "SCRIPT-V2001".to_string(),
                title: "Scripts without async/defer".to_string(),
                description: format!("{} external script(s) lack both async and defer, blocking page rendering: {}.", blocking.len(), examples),
                url: url.clone(),
                recommendation: "Add the async attribute to independent scripts or defer to scripts that must execute in order.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Performance: Font Display V2 — web fonts without font-display
// ---------------------------------------------------------------------------

pub struct FontDisplayAnalyzerV2;

impl Default for FontDisplayAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl FontDisplayAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FontDisplayAnalyzerV2 {
    fn name(&self) -> &str {
        "font-display-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        let font_extensions = [".woff2", ".woff", ".ttf", ".otf", ".eot"];
        let total_fonts: usize = ctx
            .page
            .styles
            .iter()
            .filter_map(|s| s.href.as_ref())
            .filter(|href| {
                let lower = href.to_lowercase();
                font_extensions.iter().any(|ext| lower.contains(ext))
            })
            .count()
            + font_extensions
                .iter()
                .map(|ext| body.matches(ext).count())
                .sum::<usize>();
        if total_fonts > 0 {
            let has_font_display = body.contains("font-display:");
            if !has_font_display {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Performance,
                    code: "FONT-V2001".to_string(),
                    title: "Web fonts without font-display".to_string(),
                    description: format!("Page loads {total_fonts} font file(s) but no font-display property was found. Without font-display:swap, text may be invisible while web fonts load (Flash of Invisible Text)."),
                    url: url.clone(),
                    recommendation: "Add font-display:swap to @font-face declarations to ensure text remains visible during font loading.".into(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// New media analyzer tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod new_media_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedImage, ParsedPage, ScriptInfo, StyleInfo};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(
        page: &'a ParsedPage,
        status: Option<u16>,
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // ThirdPartyResourceAnalyzer tests

    #[test]
    fn test_third_party_all_first_party() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![ScriptInfo {
            src: Some("https://example.com/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ThirdPartyResourceAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_third_party_over_threshold() {
        let mut page = make_page("https://example.com");
        page.scripts = (0..10)
            .map(|i| ScriptInfo {
                src: Some(format!("https://cdn{i}.com/lib.js")),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            })
            .collect();
        page.scripts.push(ScriptInfo {
            src: Some("https://example.com/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        });
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ThirdPartyResourceAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "THIRDPARTY001"));
    }

    #[test]
    fn test_third_party_no_resources() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ThirdPartyResourceAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_third_party_name() {
        assert_eq!(
            ThirdPartyResourceAnalyzer::new().name(),
            "third-party-resources"
        );
    }

    #[test]
    fn test_third_party_under_threshold() {
        let mut page = make_page("https://example.com");
        page.scripts = (0..10)
            .map(|i| ScriptInfo {
                src: Some(format!("https://example.com/lib{i}.js")),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ThirdPartyResourceAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_third_party_mixed_scripts_and_styles() {
        let mut page = make_page("https://example.com");
        page.scripts = (0..5)
            .map(|i| ScriptInfo {
                src: Some(format!("https://cdn{i}.com/lib.js")),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
                is_module: false,
            })
            .collect();
        page.styles = (0..5)
            .map(|i| StyleInfo {
                href: Some(format!("https://cdn{i}.com/style.css")),
                media: None,
                is_inline: false,
                has_integrity: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), None);
        // 10 external / 10 total = 100%
        assert!(ThirdPartyResourceAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "THIRDPARTY001"));
    }

    #[test]
    fn test_third_party_default() {
        let _ = ThirdPartyResourceAnalyzer::default();
    }

    // BlockingStyleAnalyzer tests

    #[test]
    fn test_blocking_style_no_styles() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        assert!(BlockingStyleAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_blocking_style_few_styles() {
        let mut page = make_page("https://example.com");
        page.styles = (0..3)
            .map(|_| StyleInfo {
                href: Some("https://example.com/style.css".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), None);
        assert!(BlockingStyleAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_blocking_style_many_styles() {
        let mut page = make_page("https://example.com");
        page.styles = (0..5)
            .map(|_| StyleInfo {
                href: Some("https://example.com/style.css".to_string()),
                media: None,
                is_inline: false,
                has_integrity: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), Some("<html></html>"));
        assert!(BlockingStyleAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "BLOCKSTYLE001"));
    }

    #[test]
    fn test_blocking_style_non_blocking_media() {
        let mut page = make_page("https://example.com");
        page.styles = (0..5)
            .map(|_| StyleInfo {
                href: Some("https://example.com/style.css".to_string()),
                media: Some("print".to_string()),
                is_inline: false,
                has_integrity: false,
            })
            .collect();
        let ctx = make_ctx(&page, Some(200), None);
        // print media is not blocking
        assert!(BlockingStyleAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_blocking_style_name() {
        assert_eq!(BlockingStyleAnalyzer::new().name(), "blocking-style");
    }

    #[test]
    fn test_blocking_style_default() {
        let _ = BlockingStyleAnalyzer::default();
    }

    #[test]
    fn test_blocking_style_html_pattern() {
        let page = make_page("https://example.com");
        let body = r#"<html><head>
            <link rel="stylesheet" href="/a.css">
            <link rel="stylesheet" href="/b.css">
            <link rel="stylesheet" href="/c.css">
            <link rel="stylesheet" href="/d.css">
        </head></html>"#;
        let ctx = make_ctx(&page, Some(200), Some(body));
        assert!(BlockingStyleAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "BLOCKSTYLE001"));
    }

    // ImageDimensionMissingAnalyzer tests

    #[test]
    fn test_image_dim_all_have_dimensions() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "https://example.com/img.jpg".to_string(),
            alt: "Test".to_string(),
            width: Some(100),
            height: Some(100),
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ImageDimensionMissingAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_image_dim_missing_dimensions() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "https://example.com/img.jpg".to_string(),
            alt: "Test".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ImageDimensionMissingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "IMGDIM001"));
    }

    #[test]
    fn test_image_dim_partial_dimensions() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "https://example.com/img.jpg".to_string(),
            alt: "Test".to_string(),
            width: Some(100),
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ImageDimensionMissingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "IMGDIM001"));
    }

    #[test]
    fn test_image_dim_no_images() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), None);
        assert!(ImageDimensionMissingAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_image_dim_name() {
        assert_eq!(
            ImageDimensionMissingAnalyzer::new().name(),
            "image-dimension-missing"
        );
    }

    #[test]
    fn test_image_dim_default() {
        let _ = ImageDimensionMissingAnalyzer::default();
    }
}
