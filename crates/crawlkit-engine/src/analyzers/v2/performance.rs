#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
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
    clippy::manual_contains,
    clippy::collapsible_match,
    clippy::redundant_clone,
    clippy::useless_conversion
)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct PreconnectHintValidator;
impl Default for PreconnectHintValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PreconnectHintValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PreconnectHintValidator {
    fn name(&self) -> &str {
        "preconnect-hint-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // Inspect actual <link rel> elements, not raw text: a comment or
            // code sample mentioning rel="preconnect" is not a hint, and
            // single-quoted / multi-value rel attributes are still valid.
            let sel = scraper::Selector::parse("link").expect("static selector");
            let has_preconnect = scraper::Html::parse_document(body)
                .select(&sel)
                .any(|el| {
                    el.value()
                        .attr("rel")
                        .map_or(false, |r| r.split_whitespace().any(|t| t == "preconnect"))
                });
            let external = ctx.page.links.iter().filter(|l| l.is_external).count();
            if external > 3 && !has_preconnect {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Performance,
                    code: "PRECON-V5001".to_string(),
                    title: "No preconnect hints".to_string(),
                    description: format!("{external} external origins, no preconnect."),
                    url: url.clone(),
                    recommendation: "Add <link rel='preconnect'> for critical origins.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct DnsPrefetchHintValidator;
impl Default for DnsPrefetchHintValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl DnsPrefetchHintValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for DnsPrefetchHintValidator {
    fn name(&self) -> &str {
        "dns-prefetch-hint-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // See PreconnectHintValidator: match real <link rel> elements,
            // including single-quoted and multi-value rel attributes.
            let sel = scraper::Selector::parse("link").expect("static selector");
            let has_prefetch = scraper::Html::parse_document(body)
                .select(&sel)
                .any(|el| {
                    el.value()
                        .attr("rel")
                        .map_or(false, |r| {
                            r.split_whitespace().any(|t| t == "dns-prefetch")
                        })
                });
            let external = ctx.page.links.iter().filter(|l| l.is_external).count();
            if external > 5 && !has_prefetch {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Performance,
                    code: "DNSPREFETCH-V5001".to_string(),
                    title: "No dns-prefetch hints".to_string(),
                    description: format!("{external} external origins, no dns-prefetch."),
                    url: url.clone(),
                    recommendation: "Add <link rel='dns-prefetch'> for external origins."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct ScriptAsyncDeferValidator;
impl Default for ScriptAsyncDeferValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ScriptAsyncDeferValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ScriptAsyncDeferValidator {
    fn name(&self) -> &str {
        "script-async-defer-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let blocking = ctx
            .page
            .scripts
            .iter()
            // ES modules (<script type="module">) are deferred by default
            // and never block parsing, so they are not render-blocking.
            .filter(|s| s.src.is_some() && !s.r#async && !s.defer && !s.is_module)
            .count();
        if blocking > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Performance,
                code: "SCRIPTBLK-V5001".to_string(),
                title: "Blocking scripts detected".to_string(),
                description: format!("{blocking} external script(s) without async/defer."),
                url: url.clone(),
                recommendation: "Add async or defer to external scripts.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageLazyLoadingValidatorV5;
impl Default for ImageLazyLoadingValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageLazyLoadingValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageLazyLoadingValidatorV5 {
    fn name(&self) -> &str {
        "image-lazy-loading-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let total_images = ctx.page.images.len();
        let lazy_images = ctx.page.images.iter().filter(|i| i.is_lazy_loaded).count();
        if total_images > 3 && lazy_images == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMGLAZY-V5001".to_string(),
                title: "No lazy loading on images".to_string(),
                description: format!("{total_images} images without lazy loading."),
                url: url.clone(),
                recommendation: "Add loading='lazy' to below-fold images.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageModernFormatValidator;
impl Default for ImageModernFormatValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageModernFormatValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageModernFormatValidator {
    fn name(&self) -> &str {
        "image-modern-format-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.images.is_empty() {
            return findings;
        }
        let total = ctx.page.images.len();
        let modern = ctx
            .page
            .images
            .iter()
            .filter(|i| {
                let src = i.src.to_lowercase();
                src.ends_with(".webp")
                    || src.ends_with(".avif")
                    || src.contains(".webp?")
                    || src.contains(".avif?")
            })
            .count();
        if total > 3 && modern == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMGFMT-V5001".to_string(),
                title: "No modern image formats".to_string(),
                description: format!("{total} images, none use WebP/AVIF."),
                url: url.clone(),
                recommendation: "Use WebP or AVIF for better compression.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageDimensionsValidatorV5;
impl Default for ImageDimensionsValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageDimensionsValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageDimensionsValidatorV5 {
    fn name(&self) -> &str {
        "image-dimensions-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let missing = ctx
            .page
            .images
            .iter()
            .filter(|i| i.width.is_none() || i.height.is_none())
            .count();
        if missing > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Performance,
                code: "IMGDIM-V5001".to_string(),
                title: "Images missing dimensions".to_string(),
                description: format!("{missing} image(s) lack width/height."),
                url: url.clone(),
                recommendation: "Add width and height attributes to prevent layout shift."
                    .to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// V6 Content Validators (1-40)
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
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
        page: &'a crate::parser::ParsedPage,
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: Some(200),
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

    #[test]
    fn test_preconnect_v5() {
        assert!(PreconnectHintValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_preconnect_v5_needed() {
        let mut p = make_page("https://example.com");
        p.links = vec![
            crate::parser::ExtractedLink {
                href: "https://a.com".into(),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://b.com".into(),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://c.com".into(),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://d.com".into(),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let f = PreconnectHintValidator::new().analyze(&make_ctx(
            &p,
            Some("<html><head></head><body></body></html>"),
        ));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_dns_prefetch_v5() {
        assert!(DnsPrefetchHintValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_script_async_v5() {
        assert!(ScriptAsyncDeferValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_script_async_v5_blocking() {
        let mut p = make_page("https://example.com");
        p.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.com/lib.js".into()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let f = ScriptAsyncDeferValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_script_async_v5_ok() {
        let mut p = make_page("https://example.com");
        p.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.com/lib.js".into()),
            r#async: true,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        assert!(ScriptAsyncDeferValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_image_lazy_v5() {
        assert!(ImageLazyLoadingValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_image_format_v5() {
        assert!(ImageModernFormatValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_image_format_v5_all_jpg() {
        let mut p = make_page("https://example.com");
        p.images = vec![
            crate::parser::ExtractedImage {
                src: "https://example.com/1.jpg".into(),
                alt: "".into(),
                has_alt: false,
                width: None,
                height: None,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            crate::parser::ExtractedImage {
                src: "https://example.com/2.jpg".into(),
                alt: "".into(),
                has_alt: false,
                width: None,
                height: None,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            crate::parser::ExtractedImage {
                src: "https://example.com/3.jpg".into(),
                alt: "".into(),
                has_alt: false,
                width: None,
                height: None,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            crate::parser::ExtractedImage {
                src: "https://example.com/4.jpg".into(),
                alt: "".into(),
                has_alt: false,
                width: None,
                height: None,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let f = ImageModernFormatValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_image_dims_v5() {
        let mut p = make_page("https://example.com");
        p.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/1.jpg".into(),
            alt: "".into(),
            has_alt: false,
            width: None,
            height: None,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let f = ImageDimensionsValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_image_dims_v5_ok() {
        let mut p = make_page("https://example.com");
        p.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/1.jpg".into(),
            alt: "".into(),
            has_alt: false,
            width: Some(100),
            height: Some(100),
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        assert!(ImageDimensionsValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V6 Content Validators Tests =====

    // ---------------------------------------------------------------------------
    // Regression: <script type="module"> is deferred by default and must not
    // be reported as render-blocking.
    // ---------------------------------------------------------------------------
    #[test]
    fn test_script_async_v5_module_not_blocking() {
        let mut p = make_page("https://example.com");
        p.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.com/app.mjs".into()),
            r#async: false,
            defer: false,
            script_type: Some("module".into()),
            has_integrity: false,
            is_module: true,
        }];
        assert!(ScriptAsyncDeferValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ---------------------------------------------------------------------------
    // Regression: resource hints must be detected from real <link rel>
    // elements — single-quoted and multi-value rel attributes count, while
    // the same text inside comments or code samples does not.
    // ---------------------------------------------------------------------------
    #[test]
    fn test_preconnect_v5_detects_single_quoted_rel() {
        let mut p = make_page("https://example.com");
        p.links = (0..4)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://cdn{i}.com"),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let body = r#"<html><head><link rel='preconnect' href='https://cdn0.com'></head><body></body></html>"#;
        assert!(PreconnectHintValidator::new()
            .analyze(&make_ctx(&p, Some(body)))
            .is_empty());
    }
    #[test]
    fn test_preconnect_v5_ignores_text_mention() {
        let mut p = make_page("https://example.com");
        p.links = (0..4)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://cdn{i}.com"),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let body = r#"<html><body><!-- TODO: add <link rel="preconnect"> hints --></body></html>"#;
        let f = PreconnectHintValidator::new().analyze(&make_ctx(&p, Some(body)));
        assert!(f.iter().any(|x| x.code == "PRECON-V5001"));
    }
    #[test]
    fn test_dns_prefetch_v5_detects_single_quoted_rel() {
        let mut p = make_page("https://example.com");
        p.links = (0..6)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://host{i}.com"),
                text: "".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let body = r#"<html><head><link rel='dns-prefetch' href='https://host0.com'></head><body></body></html>"#;
        assert!(DnsPrefetchHintValidator::new()
            .analyze(&make_ctx(&p, Some(body)))
            .is_empty());
    }
}
