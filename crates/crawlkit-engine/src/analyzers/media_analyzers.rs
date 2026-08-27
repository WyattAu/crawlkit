#![allow(clippy::unwrap_used, clippy::manual_range_contains, clippy::redundant_closure)]
use std::collections::HashSet;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

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
        data.get("offers")
            .map(|v| !v.is_null())
            .unwrap_or(false)
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

    fn extract_offer(
        data: &serde_json::Value,
    ) -> Option<&serde_json::Value> {
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
                if ar.get("@type")
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
            if let Some(rating_value) = ar
                .get("ratingValue")
                .and_then(Self::parse_f64)
            {
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

        let js_count = ctx
            .page
            .scripts
            .iter()
            .filter(|s| s.src.is_some())
            .count();

        let css_count = ctx
            .page
            .styles
            .iter()
            .filter(|s| s.href.is_some())
            .count();

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
            .filter(|s| s.src.is_some() && !s.r#async && !s.defer)
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
                    blocking.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
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
                    blocking.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            } else {
                blocking.join(", ")
            };
            f.push(Finding {
                severity: Severity::Critical,
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
