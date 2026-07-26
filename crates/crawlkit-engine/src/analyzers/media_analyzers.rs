use crate::types::{IssueCategory, Severity};
use crate::CrawlConfig;

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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.images.is_empty() {
            return findings;
        }

        let mut total_lazy = 0u32;
        let mut total_with_dimensions = 0u32;
        let mut non_modern_formats = Vec::new();

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
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMG004".to_string(),
                title: "Images missing dimensions".to_string(),
                description: format!(
                    "{} of {} images are missing width/height attributes.",
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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
