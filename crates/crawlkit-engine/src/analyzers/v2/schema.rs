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
    clippy::manual_contains,
    clippy::collapsible_match,
    clippy::redundant_clone,
    clippy::useless_conversion
)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct OrganizationUrlValidatorV5;
impl Default for OrganizationUrlValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl OrganizationUrlValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for OrganizationUrlValidatorV5 {
    fn name(&self) -> &str {
        "organization-url-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization")
                && sd.r#type.as_deref() != Some("LocalBusiness")
            {
                continue;
            }
            if let Some(org) = sd.data.as_object() {
                if !org.contains_key("url") && !org.contains_key("@id") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "ORGURL-V5001".to_string(),
                        title: "Organization missing URL".to_string(),
                        description: "Organization has no url or @id.".to_string(),
                        url: url.clone(),
                        recommendation: "Add url to Organization schema.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct OrganizationLogoUrlValidator;
impl Default for OrganizationLogoUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl OrganizationLogoUrlValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for OrganizationLogoUrlValidator {
    fn name(&self) -> &str {
        "organization-logo-url-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") {
                continue;
            }
            if let Some(logo) = sd.data.get("logo") {
                if logo.is_string() {
                    continue;
                }
                if let Some(obj) = logo.as_object() {
                    if obj
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map_or(true, |s| s.is_empty())
                    {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "ORGLOGO-V5001".to_string(),
                            title: "Organization logo missing URL".to_string(),
                            description: "Logo ImageObject has no url.".to_string(),
                            url: url.clone(),
                            recommendation: "Add url to logo ImageObject.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct OrganizationContactValidator;
impl Default for OrganizationContactValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl OrganizationContactValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for OrganizationContactValidator {
    fn name(&self) -> &str {
        "organization-contact-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") {
                continue;
            }
            if sd.data.get("contactPoint").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "ORGCONT-V5001".to_string(),
                    title: "Organization missing contactPoint".to_string(),
                    description: "No contactPoint in Organization.".to_string(),
                    url: url.clone(),
                    recommendation: "Add contactPoint with contactType.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PersonUrlValidator;
impl Default for PersonUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PersonUrlValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PersonUrlValidator {
    fn name(&self) -> &str {
        "person-url-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Person") {
                continue;
            }
            if sd.data.get("url").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERSURL-V5001".to_string(),
                    title: "Person missing url".to_string(),
                    description: "Person schema has no url.".to_string(),
                    url: url.clone(),
                    recommendation: "Add url to Person schema.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct JobPostingEmploymentTypeValidator;
impl Default for JobPostingEmploymentTypeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl JobPostingEmploymentTypeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for JobPostingEmploymentTypeValidator {
    fn name(&self) -> &str {
        "job-posting-employment-type-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let valid_types = [
            "FULL_TIME",
            "PART_TIME",
            "CONTRACT",
            "TEMPORARY",
            "INTERN",
            "VOLUNTEER",
            "PER_DIEM",
            "OTHER",
        ];
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            if let Some(et) = sd.data.get("employmentType").and_then(|v| v.as_str()) {
                if !valid_types.contains(&et) {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "JOBEMP-V5001".to_string(), title: "Invalid employmentType".to_string(), description: format!("'{et}' is not a valid Schema.org employmentType."), url: url.clone(), recommendation: "Use one of: FULL_TIME, PART_TIME, CONTRACT, TEMPORARY, INTERN, VOLUNTEER, PER_DIEM, OTHER.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct JobPostingSalaryCurrencyValidator;
impl Default for JobPostingSalaryCurrencyValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl JobPostingSalaryCurrencyValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for JobPostingSalaryCurrencyValidator {
    fn name(&self) -> &str {
        "job-posting-salary-currency-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            if let Some(salary) = sd.data.get("baseSalary") {
                if let Some(obj) = salary.as_object() {
                    if let Some(currency) = obj.get("currency") {
                        if let Some(c) = currency.as_str() {
                            if c.len() != 3 || !c.chars().all(|ch| ch.is_ascii_uppercase()) {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Schema,
                                    code: "JOBCURR-V5001".to_string(),
                                    title: "Invalid salary currency".to_string(),
                                    description: format!("'{c}' is not a valid ISO 4217 code."),
                                    url: url.clone(),
                                    recommendation: "Use a 3-letter ISO 4217 currency code."
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

pub struct JobPostingValidThroughValidator;
impl Default for JobPostingValidThroughValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl JobPostingValidThroughValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for JobPostingValidThroughValidator {
    fn name(&self) -> &str {
        "job-posting-valid-through-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            if sd
                .data
                .get("validThrough")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    // validThrough is recommended (not required) by Google's
                    // JobPosting guidelines.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "JOBVT-V5001".to_string(),
                    title: "JobPosting missing validThrough".to_string(),
                    description: "No validThrough date.".to_string(),
                    url: url.clone(),
                    recommendation: "Add validThrough to indicate expiration.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CourseDescriptionValidator;
impl Default for CourseDescriptionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CourseDescriptionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CourseDescriptionValidator {
    fn name(&self) -> &str {
        "course-description-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            if sd
                .data
                .get("description")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "COURSEDESC-V5001".to_string(),
                    title: "Course missing description".to_string(),
                    description: "No description in Course schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a description to the Course.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CourseProviderNameValidator;
impl Default for CourseProviderNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CourseProviderNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CourseProviderNameValidator {
    fn name(&self) -> &str {
        "course-provider-name-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            if let Some(provider) = sd.data.get("provider") {
                if let Some(obj) = provider.as_object() {
                    if obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map_or(true, |s| s.is_empty())
                    {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "COURSEPV-V5001".to_string(),
                            title: "Course provider missing name".to_string(),
                            description: "Provider object has no name.".to_string(),
                            url: url.clone(),
                            recommendation: "Add name to provider.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct RecipePrepTimeValidator;
impl Default for RecipePrepTimeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RecipePrepTimeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RecipePrepTimeValidator {
    fn name(&self) -> &str {
        "recipe-prep-time-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") {
                continue;
            }
            if sd
                .data
                .get("prepTime")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RECIPEPT-V5001".to_string(),
                    title: "Recipe missing prepTime".to_string(),
                    description: "No prepTime in Recipe schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add prepTime in ISO 8601 format.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct RecipeIngredientsValidator;
impl Default for RecipeIngredientsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RecipeIngredientsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RecipeIngredientsValidator {
    fn name(&self) -> &str {
        "recipe-ingredients-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") {
                continue;
            }
            if let Some(ingredients) = sd.data.get("recipeIngredient") {
                if let Some(arr) = ingredients.as_array() {
                    if arr.is_empty() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "RECIPEING-V5001".to_string(),
                            title: "Recipe has empty ingredients".to_string(),
                            description: "recipeIngredient array is empty.".to_string(),
                            url: url.clone(),
                            recommendation: "List all recipe ingredients.".to_string(),
                        });
                    }
                }
            } else {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RECIPEING-V5002".to_string(),
                    title: "Recipe missing ingredients".to_string(),
                    description: "No recipeIngredient field.".to_string(),
                    url: url.clone(),
                    recommendation: "Add recipeIngredient array.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ProductPriceValidator;
impl Default for ProductPriceValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ProductPriceValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ProductPriceValidator {
    fn name(&self) -> &str {
        "product-price-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") {
                continue;
            }
            if let Some(offers) = sd.data.get("offers") {
                let check_offer = |offer: &serde_json::Value| {
                    if let Some(obj) = offer.as_object() {
                        if obj
                            .get("price")
                            .and_then(|v| v.as_str())
                            .map_or(true, |s| s.is_empty())
                            && obj.get("price").and_then(|v| v.as_f64()).is_none()
                        {
                            return true;
                        }
                        if obj
                            .get("priceCurrency")
                            .and_then(|v| v.as_str())
                            .map_or(true, |s| s.is_empty())
                        {
                            return true;
                        }
                    }
                    false
                };
                if offers.is_object() && check_offer(offers) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "PRODPRICE-V5001".to_string(),
                        title: "Product offer missing price/currency".to_string(),
                        description: "Offer lacks price or priceCurrency.".to_string(),
                        url: url.clone(),
                        recommendation: "Add price and priceCurrency to offer.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct ProductImageValidatorV5;
impl Default for ProductImageValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl ProductImageValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ProductImageValidatorV5 {
    fn name(&self) -> &str {
        "product-image-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") {
                continue;
            }
            if sd
                .data
                .get("image")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                if let Some(img) = sd.data.get("image") {
                    if img.is_array() && img.as_array().map_or(true, |a| a.is_empty()) {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PRODIMG-V5001".to_string(),
                            title: "Product has empty image array".to_string(),
                            description: "image array is empty.".to_string(),
                            url: url.clone(),
                            recommendation: "Add product images.".to_string(),
                        });
                    }
                } else {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "PRODIMG-V5002".to_string(),
                        title: "Product missing image".to_string(),
                        description: "No image in Product schema.".to_string(),
                        url: url.clone(),
                        recommendation: "Add an image to the Product.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct BreadcrumbItemCountValidator;
impl Default for BreadcrumbItemCountValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BreadcrumbItemCountValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BreadcrumbItemCountValidator {
    fn name(&self) -> &str {
        "breadcrumb-item-count-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("BreadcrumbList") {
                continue;
            }
            if let Some(items) = sd.data.get("itemListElement") {
                if let Some(arr) = items.as_array() {
                    if arr.len() < 2 {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Schema,
                            code: "BREADCRITM-V5001".to_string(),
                            title: "Breadcrumb has too few items".to_string(),
                            description: format!("{} item(s), recommend 2+.", arr.len()),
                            url: url.clone(),
                            recommendation: "Add more breadcrumb levels.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct BreadcrumbUrlValidator;
impl Default for BreadcrumbUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BreadcrumbUrlValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BreadcrumbUrlValidator {
    fn name(&self) -> &str {
        "breadcrumb-url-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("BreadcrumbList") {
                continue;
            }
            if let Some(items) = sd.data.get("itemListElement") {
                if let Some(arr) = items.as_array() {
                    for item in arr {
                        if let Some(obj) = item.as_object() {
                            if let Some(item_url) = obj
                                .get("item")
                                .and_then(|v| v.get("@id").or(Some(v)))
                                .and_then(|v| v.as_str())
                            {
                                if !item_url.starts_with("http://")
                                    && !item_url.starts_with("https://")
                                {
                                    findings.push(Finding {
                                        severity: Severity::Warning,
                                        category: IssueCategory::Schema,
                                        code: "BREADURL-V5001".to_string(),
                                        title: "Breadcrumb has relative URL".to_string(),
                                        description: format!("URL '{item_url}' is not absolute."),
                                        url: url.clone(),
                                        recommendation: "Use absolute URLs in breadcrumbs."
                                            .to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct EventOrganizerValidatorV5;
impl Default for EventOrganizerValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl EventOrganizerValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for EventOrganizerValidatorV5 {
    fn name(&self) -> &str {
        "event-organizer-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            if let Some(organizer) = sd.data.get("organizer") {
                if let Some(obj) = organizer.as_object() {
                    if obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map_or(true, |s| s.is_empty())
                    {
                        findings.push(Finding {
                            // organizer is recommended (not required) by
                            // Google's Event guidelines.
                            severity: Severity::Info,
                            category: IssueCategory::Schema,
                            code: "EVTORG-V5001".to_string(),
                            title: "Event organizer missing name".to_string(),
                            description: "Organizer has no name.".to_string(),
                            url: url.clone(),
                            recommendation: "Add name to organizer.".to_string(),
                        });
                    }
                }
            } else {
                findings.push(Finding {
                    // organizer is recommended (not required) by Google's
                    // Event guidelines.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "EVTORG-V5002".to_string(),
                    title: "Event missing organizer".to_string(),
                    description: "No organizer in Event schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add an organizer to the Event.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct EventPerformerValidator;
impl Default for EventPerformerValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl EventPerformerValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for EventPerformerValidator {
    fn name(&self) -> &str {
        "event-performer-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            if sd.data.get("performer").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "EVTPERF-V5001".to_string(),
                    title: "Event missing performer".to_string(),
                    description: "No performer in Event schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add performer information.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct VideoThumbnailValidator;
impl Default for VideoThumbnailValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl VideoThumbnailValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for VideoThumbnailValidator {
    fn name(&self) -> &str {
        "video-thumbnail-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "VideoObject" {
                continue;
            }
            if sd
                .data
                .get("thumbnailUrl")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "VIDTHUMB-V5001".to_string(),
                    title: "Video missing thumbnailUrl".to_string(),
                    description: "No thumbnailUrl in VideoObject.".to_string(),
                    url: url.clone(),
                    recommendation: "Add thumbnailUrl for video preview.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct VideoDurationFormatValidator;
impl Default for VideoDurationFormatValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl VideoDurationFormatValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for VideoDurationFormatValidator {
    fn name(&self) -> &str {
        "video-duration-format-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "VideoObject" {
                continue;
            }
            if let Some(dur) = sd.data.get("duration").and_then(|v| v.as_str()) {
                if !dur.starts_with("PT") && !dur.starts_with("P") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "VIDDURFMT-V5001".to_string(),
                        title: "Video duration not ISO 8601".to_string(),
                        description: format!("'{dur}' is not ISO 8601 duration format."),
                        url: url.clone(),
                        recommendation: "Use ISO 8601 duration (e.g., PT1H30M).".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct SoftwareOffersValidatorV5;
impl Default for SoftwareOffersValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl SoftwareOffersValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SoftwareOffersValidatorV5 {
    fn name(&self) -> &str {
        "software-offers-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SoftwareApplication" {
                continue;
            }
            if sd.data.get("offers").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SOFTOFF-V5001".to_string(),
                    title: "Software missing offers".to_string(),
                    description: "No offers in SoftwareApplication.".to_string(),
                    url: url.clone(),
                    recommendation: "Add offers with price information.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SoftwareScreenshotValidator;
impl Default for SoftwareScreenshotValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SoftwareScreenshotValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SoftwareScreenshotValidator {
    fn name(&self) -> &str {
        "software-screenshot-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SoftwareApplication" {
                continue;
            }
            if sd
                .data
                .get("screenshot")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SOFTSS-V5001".to_string(),
                    title: "Software missing screenshot".to_string(),
                    description: "No screenshot in SoftwareApplication.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a screenshot URL.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct FAQAnswerLengthValidator;
impl Default for FAQAnswerLengthValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FAQAnswerLengthValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FAQAnswerLengthValidator {
    fn name(&self) -> &str {
        "faq-answer-length-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "FAQPage" {
                continue;
            }
            if let Some(entities) = sd.data.get("mainEntity").and_then(|v| v.as_array()) {
                let mut short_answers = 0;
                for entity in entities {
                    if let Some(answer) = entity.get("acceptedAnswer") {
                        if let Some(text) = answer.get("text").and_then(|v| v.as_str()) {
                            if text.len() < 50 {
                                short_answers += 1;
                            }
                        }
                    }
                }
                if short_answers > 0 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Content,
                        code: "FAQANSLEN-V5001".to_string(),
                        title: "FAQ answers too short".to_string(),
                        description: format!("{short_answers} answer(s) under 50 chars."),
                        url: url.clone(),
                        recommendation: "Provide substantive FAQ answers (50+ chars).".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct HowToNameValidator;
impl Default for HowToNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HowToNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HowToNameValidator {
    fn name(&self) -> &str {
        "howto-name-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HowTo" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "HOWTONAME-V5001".to_string(),
                    title: "HowTo missing name".to_string(),
                    description: "No name in HowTo schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a descriptive name to HowTo.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HowToStepDescriptionValidator;
impl Default for HowToStepDescriptionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HowToStepDescriptionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HowToStepDescriptionValidator {
    fn name(&self) -> &str {
        "howto-step-desc-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HowTo" {
                continue;
            }
            if let Some(steps) = sd.data.get("step") {
                let step_list = if let Some(arr) = steps.as_array() {
                    arr
                } else {
                    std::slice::from_ref(steps)
                };
                let mut missing_desc = 0;
                for step in step_list {
                    if step
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map_or(true, |s| s.is_empty())
                        && step
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map_or(true, |s| s.is_empty())
                    {
                        missing_desc += 1;
                    }
                }
                if missing_desc > 0 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "HOWTOSTEP-V5001".to_string(),
                        title: "HowTo steps missing description".to_string(),
                        description: format!("{missing_desc} step(s) have no text or name."),
                        url: url.clone(),
                        recommendation: "Add descriptive text to each step.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct DatasetLicenseValidator;
impl Default for DatasetLicenseValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl DatasetLicenseValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for DatasetLicenseValidator {
    fn name(&self) -> &str {
        "dataset-license-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Dataset" {
                continue;
            }
            if sd.data.get("license").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "DSLICENSE-V5001".to_string(),
                    title: "Dataset missing license".to_string(),
                    description: "No license in Dataset schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add license information.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct DatasetDistributionValidator;
impl Default for DatasetDistributionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl DatasetDistributionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for DatasetDistributionValidator {
    fn name(&self) -> &str {
        "dataset-distribution-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Dataset" {
                continue;
            }
            if let Some(dist) = sd.data.get("distribution") {
                if let Some(arr) = dist.as_array() {
                    for d in arr {
                        if let Some(obj) = d.as_object() {
                            if obj
                                .get("contentUrl")
                                .and_then(|v| v.as_str())
                                .map_or(true, |s| s.is_empty())
                            {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Schema,
                                    code: "DSDIST-V5001".to_string(),
                                    title: "Dataset distribution missing contentUrl".to_string(),
                                    description: "Distribution has no contentUrl.".to_string(),
                                    url: url.clone(),
                                    recommendation: "Add contentUrl to distribution.".to_string(),
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

// =========================================================================
// Security V5 Analyzers (31-50)
// =========================================================================

pub struct CreativeWorkMissingNameValidator;
impl Default for CreativeWorkMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CreativeWorkMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CreativeWorkMissingNameValidator {
    fn name(&self) -> &str {
        "creative-work-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CWNAME-V6001".to_string(),
                    title: "CreativeWork missing name".to_string(),
                    description: "No name in CreativeWork schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to CreativeWork.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CreativeWorkMissingDescriptionValidator;
impl Default for CreativeWorkMissingDescriptionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CreativeWorkMissingDescriptionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CreativeWorkMissingDescriptionValidator {
    fn name(&self) -> &str {
        "creative-work-missing-description-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" {
                continue;
            }
            if sd
                .data
                .get("description")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CWDESC-V6002".to_string(),
                    title: "CreativeWork missing description".to_string(),
                    description: "No description in CreativeWork schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a description to CreativeWork.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CreativeWorkMissingDateCreatedValidator;
impl Default for CreativeWorkMissingDateCreatedValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CreativeWorkMissingDateCreatedValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CreativeWorkMissingDateCreatedValidator {
    fn name(&self) -> &str {
        "creative-work-missing-date-created-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" {
                continue;
            }
            if sd
                .data
                .get("dateCreated")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CWDATE-V6003".to_string(),
                    title: "CreativeWork missing dateCreated".to_string(),
                    description: "No dateCreated in CreativeWork schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a dateCreated to CreativeWork.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PlaylistMissingNumberOfItemsValidator;
impl Default for PlaylistMissingNumberOfItemsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PlaylistMissingNumberOfItemsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PlaylistMissingNumberOfItemsValidator {
    fn name(&self) -> &str {
        "playlist-missing-number-of-items-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Playlist" {
                continue;
            }
            if sd.data.get("numberOfItems").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLNUM-V6004".to_string(),
                    title: "Playlist missing numberOfItems".to_string(),
                    description: "No numberOfItems in Playlist schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add numberOfItems to Playlist.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct FoodEstablishmentMissingMenuValidator;
impl Default for FoodEstablishmentMissingMenuValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FoodEstablishmentMissingMenuValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FoodEstablishmentMissingMenuValidator {
    fn name(&self) -> &str {
        "food-establishment-missing-menu-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "FoodEstablishment" {
                continue;
            }
            if sd
                .data
                .get("menu")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    // hasMenu is a nice-to-have for local business listings;
                    // no Google rich result requires it.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "FEMENU-V6005".to_string(),
                    title: "FoodEstablishment missing menu".to_string(),
                    description: "No menu in FoodEstablishment schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a menu URL to FoodEstablishment.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct FoodEstablishmentMissingServesCuisineValidator;
impl Default for FoodEstablishmentMissingServesCuisineValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FoodEstablishmentMissingServesCuisineValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FoodEstablishmentMissingServesCuisineValidator {
    fn name(&self) -> &str {
        "food-establishment-missing-serves-cuisine-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "FoodEstablishment" {
                continue;
            }
            if sd
                .data
                .get("servesCuisine")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "FECUIS-V6006".to_string(),
                    title: "FoodEstablishment missing servesCuisine".to_string(),
                    description: "No servesCuisine in FoodEstablishment schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add servesCuisine to FoodEstablishment.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LodgingBusinessMissingStarRatingValidator;
impl Default for LodgingBusinessMissingStarRatingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LodgingBusinessMissingStarRatingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LodgingBusinessMissingStarRatingValidator {
    fn name(&self) -> &str {
        "lodging-business-missing-star-rating-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LodgingBusiness" {
                continue;
            }
            if sd.data.get("starRating").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "LBSTAR-V6007".to_string(),
                    title: "LodgingBusiness missing starRating".to_string(),
                    description: "No starRating in LodgingBusiness schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add starRating to LodgingBusiness.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LodgingBusinessMissingAmenityFeatureValidator;
impl Default for LodgingBusinessMissingAmenityFeatureValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LodgingBusinessMissingAmenityFeatureValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LodgingBusinessMissingAmenityFeatureValidator {
    fn name(&self) -> &str {
        "lodging-business-missing-amenity-feature-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LodgingBusiness" {
                continue;
            }
            if sd.data.get("amenityFeature").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "LBAMEN-V6008".to_string(),
                    title: "LodgingBusiness missing amenityFeature".to_string(),
                    description: "No amenityFeature in LodgingBusiness schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add amenityFeature to LodgingBusiness.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SportsActivityLocationMissingSportValidator;
impl Default for SportsActivityLocationMissingSportValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SportsActivityLocationMissingSportValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SportsActivityLocationMissingSportValidator {
    fn name(&self) -> &str {
        "sports-activity-location-missing-sport-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SportsActivityLocation" {
                continue;
            }
            if sd
                .data
                .get("sport")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    // sport is descriptive metadata; omitting it does not
                    // invalidate the schema or break any rich result.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SASPORT-V6009".to_string(),
                    title: "SportsActivityLocation missing sport".to_string(),
                    description: "No sport in SportsActivityLocation schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a sport property to SportsActivityLocation.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CivicStructureMissingNameValidator;
impl Default for CivicStructureMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CivicStructureMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CivicStructureMissingNameValidator {
    fn name(&self) -> &str {
        "civic-structure-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CivicStructure" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CVNAME-V6010".to_string(),
                    title: "CivicStructure missing name".to_string(),
                    description: "No name in CivicStructure schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to CivicStructure.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LandformMissingNameValidator;
impl Default for LandformMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandformMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandformMissingNameValidator {
    fn name(&self) -> &str {
        "landform-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Landform" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "LFNAME-V6011".to_string(),
                    title: "Landform missing name".to_string(),
                    description: "No name in Landform schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to Landform.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LandmarkMissingNameValidator;
impl Default for LandmarkMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkMissingNameValidator {
    fn name(&self) -> &str {
        "landmark-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LandmarksOrHistoricalBuildings" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "LMNAME-V6012".to_string(),
                    title: "Landmark missing name".to_string(),
                    description: "No name in LandmarksOrHistoricalBuildings schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to Landmark.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TouristAttractionMissingNameValidator;
impl Default for TouristAttractionMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TouristAttractionMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TouristAttractionMissingNameValidator {
    fn name(&self) -> &str {
        "tourist-attraction-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TouristAttraction" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TANAME-V6013".to_string(),
                    title: "TouristAttraction missing name".to_string(),
                    description: "No name in TouristAttraction schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to TouristAttraction.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TouristDestinationMissingNameValidator;
impl Default for TouristDestinationMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TouristDestinationMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TouristDestinationMissingNameValidator {
    fn name(&self) -> &str {
        "tourist-destination-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TouristDestination" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TDNAME-V6014".to_string(),
                    title: "TouristDestination missing name".to_string(),
                    description: "No name in TouristDestination schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to TouristDestination.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SportsEventMissingSportValidator;
impl Default for SportsEventMissingSportValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SportsEventMissingSportValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SportsEventMissingSportValidator {
    fn name(&self) -> &str {
        "sports-event-missing-sport-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SportsEvent" {
                continue;
            }
            if sd
                .data
                .get("sport")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    // sport is descriptive metadata on a sports event;
                    // omitting it does not invalidate the schema.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SESPORT-V6015".to_string(),
                    title: "SportsEvent missing sport".to_string(),
                    description: "No sport in SportsEvent schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add sport to SportsEvent.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SportsEventMissingNameValidator;
impl Default for SportsEventMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SportsEventMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SportsEventMissingNameValidator {
    fn name(&self) -> &str {
        "sports-event-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SportsEvent" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SENAME-V6016".to_string(),
                    title: "SportsEvent missing name".to_string(),
                    description: "No name in SportsEvent schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to SportsEvent.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct EducationalOrganizationMissingNameValidator;
impl Default for EducationalOrganizationMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl EducationalOrganizationMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for EducationalOrganizationMissingNameValidator {
    fn name(&self) -> &str {
        "educational-organization-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "EducationalOrganization" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "EDUNAME-V6017".to_string(),
                    title: "EducationalOrganization missing name".to_string(),
                    description: "No name in EducationalOrganization schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to EducationalOrganization.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct NGOMissingNameValidator;
impl Default for NGOMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl NGOMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for NGOMissingNameValidator {
    fn name(&self) -> &str {
        "ngo-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "NGO" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NGONAME-V6018".to_string(),
                    title: "NGO missing name".to_string(),
                    description: "No name in NGO schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to NGO.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PerformingArtsSeriesMissingNameValidator;
impl Default for PerformingArtsSeriesMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PerformingArtsSeriesMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PerformingArtsSeriesMissingNameValidator {
    fn name(&self) -> &str {
        "performing-arts-series-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "PerformingArtsSeries" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PASNAME-V6019".to_string(),
                    title: "PerformingArtsSeries missing name".to_string(),
                    description: "No name in PerformingArtsSeries schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to PerformingArtsSeries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BroadcastEventMissingNameValidator;
impl Default for BroadcastEventMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BroadcastEventMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BroadcastEventMissingNameValidator {
    fn name(&self) -> &str {
        "broadcast-event-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "BroadcastEvent" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "BENAME-V6020".to_string(),
                    title: "BroadcastEvent missing name".to_string(),
                    description: "No name in BroadcastEvent schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to BroadcastEvent.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ProductMissingBrandValidator;
impl Default for ProductMissingBrandValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ProductMissingBrandValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ProductMissingBrandValidator {
    fn name(&self) -> &str {
        "product-missing-brand-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" {
                continue;
            }
            if sd.data.get("brand").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PRODBRAND-V6021".to_string(),
                    title: "Product missing brand".to_string(),
                    description: "No brand in Product schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add brand to Product.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ProductMissingCategoryValidator;
impl Default for ProductMissingCategoryValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ProductMissingCategoryValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ProductMissingCategoryValidator {
    fn name(&self) -> &str {
        "product-missing-category-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" {
                continue;
            }
            if sd
                .data
                .get("category")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PRODCAT-V6022".to_string(),
                    title: "Product missing category".to_string(),
                    description: "No category in Product schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a category to Product.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ProductMissingReviewValidator;
impl Default for ProductMissingReviewValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ProductMissingReviewValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ProductMissingReviewValidator {
    fn name(&self) -> &str {
        "product-missing-review-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" {
                continue;
            }
            if sd.data.get("review").is_none() && sd.data.get("aggregateRating").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PRODREV-V6023".to_string(),
                    title: "Product missing review/aggregateRating".to_string(),
                    description: "No review or aggregateRating in Product schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add review or aggregateRating to Product.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingAuthorValidator;
impl Default for BookMissingAuthorValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingAuthorValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingAuthorValidator {
    fn name(&self) -> &str {
        "book-missing-author-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd.data.get("author").is_none() {
                findings.push(Finding {
                    // author is recommended (not required) for Book; the
                    // schema stays valid without it.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKAUTH-V6024".to_string(),
                    title: "Book missing author".to_string(),
                    description: "No author in Book schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add author to Book.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingIsbnValidator;
impl Default for BookMissingIsbnValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingIsbnValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingIsbnValidator {
    fn name(&self) -> &str {
        "book-missing-isbn-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd
                .data
                .get("isbn")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKISBN-V6025".to_string(),
                    title: "Book missing isbn".to_string(),
                    description: "No isbn in Book schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add isbn to Book.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingDatePublishedValidator;
impl Default for BookMissingDatePublishedValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingDatePublishedValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingDatePublishedValidator {
    fn name(&self) -> &str {
        "book-missing-date-published-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd
                .data
                .get("datePublished")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKDATE-V6026".to_string(),
                    title: "Book missing datePublished".to_string(),
                    description: "No datePublished in Book schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add datePublished to Book.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MovieMissingDirectorValidator;
impl Default for MovieMissingDirectorValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MovieMissingDirectorValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MovieMissingDirectorValidator {
    fn name(&self) -> &str {
        "movie-missing-director-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" {
                continue;
            }
            if sd.data.get("director").is_none() {
                findings.push(Finding {
                    // director is recommended (not required) for Movie; the
                    // schema stays valid without it.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVDIR-V6027".to_string(),
                    title: "Movie missing director".to_string(),
                    description: "No director in Movie schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add director to Movie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MovieMissingDurationValidator;
impl Default for MovieMissingDurationValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MovieMissingDurationValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MovieMissingDurationValidator {
    fn name(&self) -> &str {
        "movie-missing-duration-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" {
                continue;
            }
            if sd
                .data
                .get("duration")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVDUR-V6028".to_string(),
                    title: "Movie missing duration".to_string(),
                    description: "No duration in Movie schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add ISO 8601 duration to Movie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MovieMissingDateCreatedValidator;
impl Default for MovieMissingDateCreatedValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MovieMissingDateCreatedValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MovieMissingDateCreatedValidator {
    fn name(&self) -> &str {
        "movie-missing-date-created-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" {
                continue;
            }
            if sd
                .data
                .get("dateCreated")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVDATE-V6029".to_string(),
                    title: "Movie missing dateCreated".to_string(),
                    description: "No dateCreated in Movie schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add dateCreated to Movie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TVSeriesMissingNumberOfSeasonsValidator;
impl Default for TVSeriesMissingNumberOfSeasonsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TVSeriesMissingNumberOfSeasonsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TVSeriesMissingNumberOfSeasonsValidator {
    fn name(&self) -> &str {
        "tv-series-missing-number-of-seasons-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TVSeries" {
                continue;
            }
            if sd.data.get("numberOfSeasons").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TVSEASON-V6030".to_string(),
                    title: "TVSeries missing numberOfSeasons".to_string(),
                    description: "No numberOfSeasons in TVSeries schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add numberOfSeasons to TVSeries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TVSeriesMissingEpisodeValidator;
impl Default for TVSeriesMissingEpisodeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TVSeriesMissingEpisodeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TVSeriesMissingEpisodeValidator {
    fn name(&self) -> &str {
        "tv-series-missing-episode-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TVSeries" {
                continue;
            }
            if sd.data.get("episode").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TVEP-V6031".to_string(),
                    title: "TVSeries missing episode".to_string(),
                    description: "No episode in TVSeries schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add episode information to TVSeries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MusicRecordingMissingByArtistValidator;
impl Default for MusicRecordingMissingByArtistValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MusicRecordingMissingByArtistValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MusicRecordingMissingByArtistValidator {
    fn name(&self) -> &str {
        "music-recording-missing-by-artist-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "MusicRecording" {
                continue;
            }
            if sd.data.get("byArtist").is_none() {
                findings.push(Finding {
                    // byArtist is a nice-to-have credit on a music recording.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MUSART-V6032".to_string(),
                    title: "MusicRecording missing byArtist".to_string(),
                    description: "No byArtist in MusicRecording schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add byArtist to MusicRecording.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MusicRecordingMissingAlbumValidator;
impl Default for MusicRecordingMissingAlbumValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MusicRecordingMissingAlbumValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MusicRecordingMissingAlbumValidator {
    fn name(&self) -> &str {
        "music-recording-missing-album-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "MusicRecording" {
                continue;
            }
            if sd.data.get("inAlbum").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MUSALB-V6033".to_string(),
                    title: "MusicRecording missing inAlbum".to_string(),
                    description: "No inAlbum in MusicRecording schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add inAlbum to MusicRecording.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ServiceMissingAreaServedValidator;
impl Default for ServiceMissingAreaServedValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ServiceMissingAreaServedValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ServiceMissingAreaServedValidator {
    fn name(&self) -> &str {
        "service-missing-area-served-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" {
                continue;
            }
            if sd.data.get("areaServed").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SVCAREA-V6034".to_string(),
                    title: "Service missing areaServed".to_string(),
                    description: "No areaServed in Service schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add areaServed to Service.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ServiceMissingProviderValidator;
impl Default for ServiceMissingProviderValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ServiceMissingProviderValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ServiceMissingProviderValidator {
    fn name(&self) -> &str {
        "service-missing-provider-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" {
                continue;
            }
            if sd.data.get("provider").is_none() {
                findings.push(Finding {
                    // provider is a nice-to-have attribution on Service.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SVCPRV-V6035".to_string(),
                    title: "Service missing provider".to_string(),
                    description: "No provider in Service schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add provider to Service.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HealthPlanMissingProviderValidator;
impl Default for HealthPlanMissingProviderValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HealthPlanMissingProviderValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HealthPlanMissingProviderValidator {
    fn name(&self) -> &str {
        "health-plan-missing-provider-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HealthPlan" {
                continue;
            }
            if sd.data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "HPPRV-V6036".to_string(),
                    title: "HealthPlan missing provider".to_string(),
                    description: "No provider in HealthPlan schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add provider to HealthPlan.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HealthPlanMissingCoverageAreaValidator;
impl Default for HealthPlanMissingCoverageAreaValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HealthPlanMissingCoverageAreaValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HealthPlanMissingCoverageAreaValidator {
    fn name(&self) -> &str {
        "health-plan-missing-coverage-area-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HealthPlan" {
                continue;
            }
            if sd.data.get("coverageArea").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "HPCOV-V6037".to_string(),
                    title: "HealthPlan missing coverageArea".to_string(),
                    description: "No coverageArea in HealthPlan schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add coverageArea to HealthPlan.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InvoiceMissingAccountValidator;
impl Default for InvoiceMissingAccountValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InvoiceMissingAccountValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InvoiceMissingAccountValidator {
    fn name(&self) -> &str {
        "invoice-missing-account-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Invoice" {
                continue;
            }
            if sd.data.get("account").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "INVACCT-V6038".to_string(),
                    title: "Invoice missing account".to_string(),
                    description: "No account in Invoice schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add account to Invoice.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InvoiceMissingPaymentDueDateValidator;
impl Default for InvoiceMissingPaymentDueDateValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InvoiceMissingPaymentDueDateValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InvoiceMissingPaymentDueDateValidator {
    fn name(&self) -> &str {
        "invoice-missing-payment-due-date-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Invoice" {
                continue;
            }
            if sd
                .data
                .get("paymentDueDate")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "INVPAY-V6039".to_string(),
                    title: "Invoice missing paymentDueDate".to_string(),
                    description: "No paymentDueDate in Invoice schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add paymentDueDate to Invoice.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermitMissingPermitNumberValidator;
impl Default for PermitMissingPermitNumberValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermitMissingPermitNumberValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermitMissingPermitNumberValidator {
    fn name(&self) -> &str {
        "permit-missing-permit-number-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Permit" {
                continue;
            }
            if sd
                .data
                .get("permitNumber")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERMNUM-V6040".to_string(),
                    title: "Permit missing permitNumber".to_string(),
                    description: "No permitNumber in Permit schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add permitNumber to Permit.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermitMissingIssuedByValidator;
impl Default for PermitMissingIssuedByValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermitMissingIssuedByValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermitMissingIssuedByValidator {
    fn name(&self) -> &str {
        "permit-missing-issued-by-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Permit" {
                continue;
            }
            if sd.data.get("issuedBy").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERMISS-V6041".to_string(),
                    title: "Permit missing issuedBy".to_string(),
                    description: "No issuedBy in Permit schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add issuedBy to Permit.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PlanMissingDescriptionValidator;
impl Default for PlanMissingDescriptionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PlanMissingDescriptionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PlanMissingDescriptionValidator {
    fn name(&self) -> &str {
        "plan-missing-description-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Plan" {
                continue;
            }
            if sd
                .data
                .get("description")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLANDESC-V6042".to_string(),
                    title: "Plan missing description".to_string(),
                    description: "No description in Plan schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a description to Plan.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PlanMissingAboutValidator;
impl Default for PlanMissingAboutValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PlanMissingAboutValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PlanMissingAboutValidator {
    fn name(&self) -> &str {
        "plan-missing-about-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Plan" {
                continue;
            }
            if sd.data.get("about").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLANABOUT-V6043".to_string(),
                    title: "Plan missing about".to_string(),
                    description: "No about in Plan schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add about to Plan.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ResearchProjectMissingAboutValidator;
impl Default for ResearchProjectMissingAboutValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ResearchProjectMissingAboutValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ResearchProjectMissingAboutValidator {
    fn name(&self) -> &str {
        "research-project-missing-about-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "ResearchProject" {
                continue;
            }
            if sd.data.get("about").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RPABOUT-V6044".to_string(),
                    title: "ResearchProject missing about".to_string(),
                    description: "No about in ResearchProject schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add about to ResearchProject.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ResearchProjectMissingFunderValidator;
impl Default for ResearchProjectMissingFunderValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ResearchProjectMissingFunderValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ResearchProjectMissingFunderValidator {
    fn name(&self) -> &str {
        "research-project-missing-funder-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "ResearchProject" {
                continue;
            }
            if sd.data.get("funder").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RPFUND-V6045".to_string(),
                    title: "ResearchProject missing funder".to_string(),
                    description: "No funder in ResearchProject schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add funder to ResearchProject.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ScheduleMissingTimezoneValidator;
impl Default for ScheduleMissingTimezoneValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ScheduleMissingTimezoneValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ScheduleMissingTimezoneValidator {
    fn name(&self) -> &str {
        "schedule-missing-timezone-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Schedule" {
                continue;
            }
            if sd
                .data
                .get("scheduleTimezone")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SCHEDTZ-V6046".to_string(),
                    title: "Schedule missing scheduleTimezone".to_string(),
                    description: "No scheduleTimezone in Schedule schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add scheduleTimezone to Schedule.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TripMissingItineraryValidator;
impl Default for TripMissingItineraryValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TripMissingItineraryValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TripMissingItineraryValidator {
    fn name(&self) -> &str {
        "trip-missing-itinerary-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Trip" {
                continue;
            }
            if sd.data.get("itinerary").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TRIPIT-V6047".to_string(),
                    title: "Trip missing itinerary".to_string(),
                    description: "No itinerary in Trip schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add itinerary to Trip.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WorkersUnionMissingNameValidator;
impl Default for WorkersUnionMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkersUnionMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WorkersUnionMissingNameValidator {
    fn name(&self) -> &str {
        "workers-union-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WorkersUnion" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WUNAME-V6048".to_string(),
                    title: "WorkersUnion missing name".to_string(),
                    description: "No name in WorkersUnion schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to WorkersUnion.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WebAPIMissingDocumentationValidator;
impl Default for WebAPIMissingDocumentationValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WebAPIMissingDocumentationValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WebAPIMissingDocumentationValidator {
    fn name(&self) -> &str {
        "webapi-missing-documentation-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebAPI" {
                continue;
            }
            if sd
                .data
                .get("documentation")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WAPIDOC-V6049".to_string(),
                    title: "WebAPI missing documentation".to_string(),
                    description: "No documentation in WebAPI schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add documentation URL to WebAPI.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WearableMissingDeviceTypeValidator;
impl Default for WearableMissingDeviceTypeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WearableMissingDeviceTypeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WearableMissingDeviceTypeValidator {
    fn name(&self) -> &str {
        "wearable-missing-device-type-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Wearable" {
                continue;
            }
            if sd
                .data
                .get("deviceType")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WEARDEV-V6050".to_string(),
                    title: "Wearable missing deviceType".to_string(),
                    description: "No deviceType in Wearable schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add deviceType to Wearable.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WebPageElementMissingNameValidator;
impl Default for WebPageElementMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WebPageElementMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WebPageElementMissingNameValidator {
    fn name(&self) -> &str {
        "webpage-element-missing-name-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebPageElement" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WPELNAME-V6051".to_string(),
                    title: "WebPageElement missing name".to_string(),
                    description: "No name in WebPageElement schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add a name to WebPageElement.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WorkerMissingJobTitleValidator;
impl Default for WorkerMissingJobTitleValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkerMissingJobTitleValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WorkerMissingJobTitleValidator {
    fn name(&self) -> &str {
        "worker-missing-job-title-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Worker" {
                continue;
            }
            if sd
                .data
                .get("jobTitle")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WORKJOB-V6052".to_string(),
                    title: "Worker missing jobTitle".to_string(),
                    description: "No jobTitle in Worker schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add jobTitle to Worker.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CreativeWorkMissingLicenseValidator;
impl Default for CreativeWorkMissingLicenseValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CreativeWorkMissingLicenseValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CreativeWorkMissingLicenseValidator {
    fn name(&self) -> &str {
        "creative-work-missing-license-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" {
                continue;
            }
            if sd
                .data
                .get("license")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CWLIC-V6053".to_string(),
                    title: "CreativeWork missing license".to_string(),
                    description: "No license in CreativeWork schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add license to CreativeWork.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ProductMissingSkuValidator;
impl Default for ProductMissingSkuValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ProductMissingSkuValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ProductMissingSkuValidator {
    fn name(&self) -> &str {
        "product-missing-sku-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" {
                continue;
            }
            if sd
                .data
                .get("sku")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PRODSKU-V6054".to_string(),
                    title: "Product missing sku".to_string(),
                    description: "No sku in Product schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add sku to Product.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingPublisherValidator;
impl Default for BookMissingPublisherValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingPublisherValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingPublisherValidator {
    fn name(&self) -> &str {
        "book-missing-publisher-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd.data.get("publisher").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKPUB-V6055".to_string(),
                    title: "Book missing publisher".to_string(),
                    description: "No publisher in Book schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add publisher to Book.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MovieMissingActorValidator;
impl Default for MovieMissingActorValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MovieMissingActorValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MovieMissingActorValidator {
    fn name(&self) -> &str {
        "movie-missing-actor-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" {
                continue;
            }
            if sd.data.get("actor").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVACT-V6056".to_string(),
                    title: "Movie missing actor".to_string(),
                    description: "No actor in Movie schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add actor information to Movie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ServiceMissingServiceTypeValidator;
impl Default for ServiceMissingServiceTypeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ServiceMissingServiceTypeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ServiceMissingServiceTypeValidator {
    fn name(&self) -> &str {
        "service-missing-service-type-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" {
                continue;
            }
            if sd
                .data
                .get("serviceType")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SVCTYPE-V6057".to_string(),
                    title: "Service missing serviceType".to_string(),
                    description: "No serviceType in Service schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add serviceType to Service.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct EventMissingStartDateValidator;
impl Default for EventMissingStartDateValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl EventMissingStartDateValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for EventMissingStartDateValidator {
    fn name(&self) -> &str {
        "event-missing-start-date-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Event" {
                continue;
            }
            if sd
                .data
                .get("startDate")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "EVTSTART-V6058".to_string(),
                    title: "Event missing startDate".to_string(),
                    description: "No startDate in Event schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add startDate to Event.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct EventMissingLocationValidator;
impl Default for EventMissingLocationValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl EventMissingLocationValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for EventMissingLocationValidator {
    fn name(&self) -> &str {
        "event-missing-location-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Event" {
                continue;
            }
            if sd.data.get("location").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "EVTLOC-V6059".to_string(),
                    title: "Event missing location".to_string(),
                    description: "No location in Event schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add location to Event.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LocalBusinessMissingGeoValidator;
impl Default for LocalBusinessMissingGeoValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LocalBusinessMissingGeoValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LocalBusinessMissingGeoValidator {
    fn name(&self) -> &str {
        "local-business-missing-geo-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LocalBusiness" {
                continue;
            }
            if sd.data.get("geo").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "LBGEO-V6060".to_string(),
                    title: "LocalBusiness missing geo".to_string(),
                    description: "No geo in LocalBusiness schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add geo coordinates to LocalBusiness.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// V6 Security Validators (41-65)
// =========================================================================

pub struct ApartmentMissingNumberOfRoomsValidator;
impl Default for ApartmentMissingNumberOfRoomsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ApartmentMissingNumberOfRoomsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ApartmentMissingNumberOfRoomsValidator {
    fn name(&self) -> &str {
        "apartment-missing-number-of-rooms-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Apartment" {
                continue;
            }
            if sd.data.get("numberOfRooms").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "APTROOM001".to_string(),
                    title: "Apartment missing numberOfRooms".to_string(),
                    description: "No numberOfRooms in Apartment schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add numberOfRooms to Apartment.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CarMissingModelValidator;
impl Default for CarMissingModelValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CarMissingModelValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CarMissingModelValidator {
    fn name(&self) -> &str {
        "car-missing-model-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Car" {
                continue;
            }
            if sd
                .data
                .get("model")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CARMODEL001".to_string(),
                    title: "Car missing model".to_string(),
                    description: "No model in Car schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add model to Car.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CarMissingManufacturerValidator;
impl Default for CarMissingManufacturerValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CarMissingManufacturerValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CarMissingManufacturerValidator {
    fn name(&self) -> &str {
        "car-missing-manufacturer-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Car" {
                continue;
            }
            if sd.data.get("manufacturer").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CARMFR001".to_string(),
                    title: "Car missing manufacturer".to_string(),
                    description: "No manufacturer in Car schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add manufacturer to Car.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MusicAlbumMissingNumberOfTracksValidator;
impl Default for MusicAlbumMissingNumberOfTracksValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MusicAlbumMissingNumberOfTracksValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MusicAlbumMissingNumberOfTracksValidator {
    fn name(&self) -> &str {
        "music-album-missing-number-of-tracks-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "MusicAlbum" {
                continue;
            }
            if sd.data.get("numberOfTracks").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MUSTRACKS001".to_string(),
                    title: "MusicAlbum missing numberOfTracks".to_string(),
                    description: "No numberOfTracks in MusicAlbum schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add numberOfTracks to MusicAlbum.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TVSeriesMissingNumberOfEpisodesValidator;
impl Default for TVSeriesMissingNumberOfEpisodesValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TVSeriesMissingNumberOfEpisodesValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TVSeriesMissingNumberOfEpisodesValidator {
    fn name(&self) -> &str {
        "tv-series-missing-number-of-episodes-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TVSeries" {
                continue;
            }
            if sd.data.get("numberOfEpisodes").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TVEPISODES001".to_string(),
                    title: "TVSeries missing numberOfEpisodes".to_string(),
                    description: "No numberOfEpisodes in TVSeries schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add numberOfEpisodes to TVSeries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MovieMissingAggregateRatingValidator;
impl Default for MovieMissingAggregateRatingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MovieMissingAggregateRatingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MovieMissingAggregateRatingValidator {
    fn name(&self) -> &str {
        "movie-missing-aggregate-rating-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" {
                continue;
            }
            if sd.data.get("aggregateRating").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVRATING001".to_string(),
                    title: "Movie missing aggregateRating".to_string(),
                    description: "No aggregateRating in Movie schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add aggregateRating to Movie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct GovernmentServiceMissingServiceAreaValidator;
impl Default for GovernmentServiceMissingServiceAreaValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl GovernmentServiceMissingServiceAreaValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for GovernmentServiceMissingServiceAreaValidator {
    fn name(&self) -> &str {
        "government-service-missing-service-area-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "GovernmentService" {
                continue;
            }
            if sd.data.get("serviceArea").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "GOVSERVICE001".to_string(),
                    title: "GovernmentService missing serviceArea".to_string(),
                    description: "No serviceArea in GovernmentService schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add serviceArea to GovernmentService.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HealthPlanMissingHealthPlanIdValidator;
impl Default for HealthPlanMissingHealthPlanIdValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HealthPlanMissingHealthPlanIdValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HealthPlanMissingHealthPlanIdValidator {
    fn name(&self) -> &str {
        "health-plan-missing-health-plan-id-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HealthPlan" {
                continue;
            }
            if sd
                .data
                .get("healthPlanId")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "HPID001".to_string(),
                    title: "HealthPlan missing healthPlanId".to_string(),
                    description: "No healthPlanId in HealthPlan schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add healthPlanId to HealthPlan.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InvoiceMissingPaymentStatusValidator;
impl Default for InvoiceMissingPaymentStatusValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl InvoiceMissingPaymentStatusValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InvoiceMissingPaymentStatusValidator {
    fn name(&self) -> &str {
        "invoice-missing-payment-status-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Invoice" {
                continue;
            }
            if sd
                .data
                .get("paymentStatus")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "INVSTATUS001".to_string(),
                    title: "Invoice missing paymentStatus".to_string(),
                    description: "No paymentStatus in Invoice schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add paymentStatus to Invoice.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermitMissingValidFromValidator;
impl Default for PermitMissingValidFromValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermitMissingValidFromValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermitMissingValidFromValidator {
    fn name(&self) -> &str {
        "permit-missing-valid-from-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Permit" {
                continue;
            }
            if sd
                .data
                .get("validFrom")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERMFROM001".to_string(),
                    title: "Permit missing validFrom".to_string(),
                    description: "No validFrom in Permit schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add validFrom to Permit.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PlanMissingBenefitsValidator;
impl Default for PlanMissingBenefitsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PlanMissingBenefitsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PlanMissingBenefitsValidator {
    fn name(&self) -> &str {
        "plan-missing-benefits-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Plan" {
                continue;
            }
            if sd.data.get("benefits").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLANBEN001".to_string(),
                    title: "Plan missing benefits".to_string(),
                    description: "No benefits in Plan schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add benefits to Plan.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ResearchProjectMissingFundingRecognizerValidator;
impl Default for ResearchProjectMissingFundingRecognizerValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ResearchProjectMissingFundingRecognizerValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ResearchProjectMissingFundingRecognizerValidator {
    fn name(&self) -> &str {
        "research-project-missing-funding-recognizer-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "ResearchProject" {
                continue;
            }
            if sd.data.get("funder").is_none() && sd.data.get("funding").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RPFUND001".to_string(),
                    title: "ResearchProject missing funding info".to_string(),
                    description: "No funder or funding in ResearchProject schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add funder or funding to ResearchProject.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ScheduleMissingRepeatFrequencyValidator;
impl Default for ScheduleMissingRepeatFrequencyValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ScheduleMissingRepeatFrequencyValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ScheduleMissingRepeatFrequencyValidator {
    fn name(&self) -> &str {
        "schedule-missing-repeat-frequency-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Schedule" {
                continue;
            }
            if sd.data.get("repeatFrequency").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SCHEDFREQ001".to_string(),
                    title: "Schedule missing repeatFrequency".to_string(),
                    description: "No repeatFrequency in Schedule schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add repeatFrequency to Schedule.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TripMissingDepartureTimeValidator;
impl Default for TripMissingDepartureTimeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TripMissingDepartureTimeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TripMissingDepartureTimeValidator {
    fn name(&self) -> &str {
        "trip-missing-departure-time-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Trip" {
                continue;
            }
            if sd
                .data
                .get("departureTime")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TRIPDEP001".to_string(),
                    title: "Trip missing departureTime".to_string(),
                    description: "No departureTime in Trip schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add departureTime to Trip.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WorkersUnionMissingMemberCountValidator;
impl Default for WorkersUnionMissingMemberCountValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkersUnionMissingMemberCountValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WorkersUnionMissingMemberCountValidator {
    fn name(&self) -> &str {
        "workers-union-missing-member-count-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WorkersUnion" {
                continue;
            }
            if sd.data.get("memberCount").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WUMEMBER001".to_string(),
                    title: "WorkersUnion missing memberCount".to_string(),
                    description: "No memberCount in WorkersUnion schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add memberCount to WorkersUnion.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WebAPIDocumentationMissingValidator;
impl Default for WebAPIDocumentationMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WebAPIDocumentationMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WebAPIDocumentationMissingValidator {
    fn name(&self) -> &str {
        "web-api-documentation-missing-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebAPI" {
                continue;
            }
            if sd
                .data
                .get("documentation")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WAPIDOCS001".to_string(),
                    title: "WebAPI missing documentation".to_string(),
                    description: "No documentation in WebAPI schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add documentation URL to WebAPI.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WearableMissingBatteryLifeValidator;
impl Default for WearableMissingBatteryLifeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WearableMissingBatteryLifeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WearableMissingBatteryLifeValidator {
    fn name(&self) -> &str {
        "wearable-missing-battery-life-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Wearable" && t != "WearableMeasurement" {
                continue;
            }
            if sd.data.get("batteryLife").is_none() && sd.data.get("batteryCharge").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WEARBATT001".to_string(),
                    title: "Wearable missing battery info".to_string(),
                    description: "No batteryLife or batteryCharge in Wearable schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add batteryLife or batteryCharge to Wearable.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WebPageElementMissingAccessibleNameValidator;
impl Default for WebPageElementMissingAccessibleNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WebPageElementMissingAccessibleNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WebPageElementMissingAccessibleNameValidator {
    fn name(&self) -> &str {
        "web-page-element-missing-accessible-name-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebPageElement" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    // WebPageElement has no dedicated rich result; an
                    // accessible name is a nice-to-have, not a failure.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WPELACC001".to_string(),
                    title: "WebPageElement missing name".to_string(),
                    description: "No name in WebPageElement schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to WebPageElement.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WorkerMissingOccupationValidator;
impl Default for WorkerMissingOccupationValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkerMissingOccupationValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WorkerMissingOccupationValidator {
    fn name(&self) -> &str {
        "worker-missing-occupation-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Worker" {
                continue;
            }
            if sd.data.get("occupation").is_none() && sd.data.get("jobTitle").is_none() {
                findings.push(Finding {
                    // occupation/jobTitle is a nice-to-have detail on Worker.
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WOCCUP001".to_string(),
                    title: "Worker missing occupation/jobTitle".to_string(),
                    description: "No occupation or jobTitle in Worker schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add occupation or jobTitle to Worker.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingBookFormatValidator;
impl Default for BookMissingBookFormatValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingBookFormatValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingBookFormatValidator {
    fn name(&self) -> &str {
        "book-missing-book-format-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd
                .data
                .get("bookFormat")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKFMT001".to_string(),
                    title: "Book missing bookFormat".to_string(),
                    description: "No bookFormat in Book schema.".to_string(),
                    url: url.clone(),
                    recommendation: "Add bookFormat to Book.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// V7 Security Validators (21-35)
// =========================================================================

pub struct DatasetMissingDescriptionValidator;
impl Default for DatasetMissingDescriptionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl DatasetMissingDescriptionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for DatasetMissingDescriptionValidator {
    fn name(&self) -> &str {
        "dataset-missing-description-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Dataset" {
                continue;
            }
            match sd.data.get("description") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "DATDESC001".to_string(), title: "Dataset missing description".to_string(), description: "No description in Dataset schema. Description helps search engines understand dataset content.".to_string(), url: url.clone(), recommendation: "Add a description to the Dataset schema.".to_string() });
                }
                Some(v) if v.as_str().map_or(false, |s| s.trim().is_empty()) => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "DATDESC001".to_string(),
                        title: "Dataset missing description".to_string(),
                        description: "Description is empty or whitespace only.".to_string(),
                        url: url.clone(),
                        recommendation: "Add a meaningful description to the Dataset schema."
                            .to_string(),
                    });
                }
                _ => {}
            }
        }
        findings
    }
}

pub struct DatasetMissingDistributionValidator;
impl Default for DatasetMissingDistributionValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl DatasetMissingDistributionValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for DatasetMissingDistributionValidator {
    fn name(&self) -> &str {
        "dataset-missing-distribution-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Dataset" {
                continue;
            }
            match sd.data.get("distribution") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding {
                        // distribution is recommended (not required) by
                        // Google's Dataset guidelines.
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "DATDIST001".to_string(),
                        title: "Dataset missing distribution".to_string(),
                        description: "No distribution in Dataset schema.".to_string(),
                        url: url.clone(),
                        recommendation: "Add distribution with DataDownload objects.".to_string(),
                    });
                }
                Some(v) => {
                    if let Some(arr) = v.as_array() {
                        if arr.is_empty() {
                            findings.push(Finding {
                                // distribution is recommended (not required)
                                // by Google's Dataset guidelines.
                                severity: Severity::Info,
                                category: IssueCategory::Schema,
                                code: "DATDIST001".to_string(),
                                title: "Dataset has empty distribution".to_string(),
                                description: "Dataset distribution array is empty.".to_string(),
                                url: url.clone(),
                                recommendation:
                                    "Add DataDownload objects with contentUrl to distribution."
                                        .to_string(),
                            });
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct HowToMissingNameValidator;
impl Default for HowToMissingNameValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HowToMissingNameValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HowToMissingNameValidator {
    fn name(&self) -> &str {
        "howto-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HowTo" {
                continue;
            }
            match sd.data.get("name") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "HOWNAME001".to_string(),
                        title: "HowTo missing name".to_string(),
                        description: "No name in HowTo schema.".to_string(),
                        url: url.clone(),
                        recommendation: "Add a descriptive name to the HowTo schema.".to_string(),
                    });
                }
                Some(v) if v.as_str().map_or(false, |s| s.trim().is_empty()) => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "HOWNAME001".to_string(),
                        title: "HowTo missing name".to_string(),
                        description: "Name is empty or whitespace only.".to_string(),
                        url: url.clone(),
                        recommendation: "Add a descriptive name to the HowTo schema.".to_string(),
                    });
                }
                _ => {}
            }
        }
        findings
    }
}

pub struct HowToMissingStepValidator;
impl Default for HowToMissingStepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HowToMissingStepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HowToMissingStepValidator {
    fn name(&self) -> &str {
        "howto-missing-step-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HowTo" {
                continue;
            }
            match sd.data.get("step") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "HOWSTEP001".to_string(),
                        title: "HowTo missing step".to_string(),
                        description: "No step in HowTo schema.".to_string(),
                        url: url.clone(),
                        recommendation: "Add step array with HowToStep objects.".to_string(),
                    });
                }
                Some(steps) => {
                    if let Some(arr) = steps.as_array() {
                        if arr.is_empty() {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "HOWSTEP001".to_string(),
                                title: "HowTo has empty step array".to_string(),
                                description: "step array is empty.".to_string(),
                                url: url.clone(),
                                recommendation: "Add HowToStep objects to the step array."
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct RecipeMissingCookTimeValidator;
impl Default for RecipeMissingCookTimeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl RecipeMissingCookTimeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RecipeMissingCookTimeValidator {
    fn name(&self) -> &str {
        "recipe-missing-cooktime-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Recipe" {
                continue;
            }
            match sd.data.get("cookTime") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "RECCOOK001".to_string(),
                        title: "Recipe missing cookTime".to_string(),
                        description: "No cookTime in Recipe schema.".to_string(),
                        url: url.clone(),
                        recommendation: "Add cookTime with an ISO 8601 duration value.".to_string(),
                    });
                }
                Some(v) if v.as_str().map_or(false, |s| s.trim().is_empty()) => {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "RECCOOK001".to_string(),
                        title: "Recipe missing cookTime".to_string(),
                        description: "cookTime is empty or whitespace only.".to_string(),
                        url: url.clone(),
                        recommendation: "Add cookTime with an ISO 8601 duration value.".to_string(),
                    });
                }
                _ => {}
            }
        }
        findings
    }
}

// =========================================================================
// V8 Security Validators (Permissions-Policy deep validators)
// =========================================================================

pub struct ApartmentMissingNumberOfRoomsValidatorV2;
impl Default for ApartmentMissingNumberOfRoomsValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ApartmentMissingNumberOfRoomsValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ApartmentMissingNumberOfRoomsValidatorV2 {
    fn name(&self) -> &str {
        "apartment-missing-number-of-rooms-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Apartment" {
                continue;
            }
            match sd.data.get("numberOfRooms") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "APTROOM-V2001".to_string(),
                        title: "Apartment missing numberOfRooms".to_string(),
                        description: "Apartment schema is missing the numberOfRooms property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add numberOfRooms to Apartment structured data."
                            .to_string(),
                    });
                }
                Some(v) if v.as_str().map_or(false, |s| s.trim().is_empty()) => {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "APTROOM-V2001".to_string(),
                        title: "Apartment has empty numberOfRooms".to_string(),
                        description: "numberOfRooms is empty or whitespace only.".to_string(),
                        url: url.clone(),
                        recommendation: "Add numberOfRooms to Apartment structured data."
                            .to_string(),
                    });
                }
                _ => {}
            }
        }
        findings
    }
}

pub struct CarMissingModelValidatorV2;
impl Default for CarMissingModelValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CarMissingModelValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CarMissingModelValidatorV2 {
    fn name(&self) -> &str {
        "car-missing-model-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Car" {
                continue;
            }
            if sd
                .data
                .get("model")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "CARMODEL-V2001".to_string(),
                    title: "Car missing model".to_string(),
                    description: "Car schema is missing the model property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add model to Car structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MusicAlbumMissingTracksValidatorV2;
impl Default for MusicAlbumMissingTracksValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MusicAlbumMissingTracksValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MusicAlbumMissingTracksValidatorV2 {
    fn name(&self) -> &str {
        "music-album-missing-tracks-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "MusicAlbum" {
                continue;
            }
            if sd.data.get("track").is_none()
                && sd.data.get("numTracks").is_none()
                && sd.data.get("numberOfTracks").is_none()
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MUSCTRACKS-V2001".to_string(),
                    title: "MusicAlbum missing tracks".to_string(),
                    description:
                        "MusicAlbum schema has no track, numTracks, or numberOfTracks property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add track or numberOfTracks to MusicAlbum.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TVSeriesMissingEpisodesValidatorV2;
impl Default for TVSeriesMissingEpisodesValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TVSeriesMissingEpisodesValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TVSeriesMissingEpisodesValidatorV2 {
    fn name(&self) -> &str {
        "tv-series-missing-episodes-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TVSeries" {
                continue;
            }
            if sd.data.get("episode").is_none() && sd.data.get("numberOfEpisodes").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TVEP-V2001".to_string(),
                    title: "TVSeries missing episodes".to_string(),
                    description: "TVSeries schema has no episode or numberOfEpisodes property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add episode or numberOfEpisodes to TVSeries.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MovieMissingDirectorValidatorV2;
impl Default for MovieMissingDirectorValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MovieMissingDirectorValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MovieMissingDirectorValidatorV2 {
    fn name(&self) -> &str {
        "movie-missing-director-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" {
                continue;
            }
            if sd.data.get("director").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVDIR-V2001".to_string(),
                    title: "Movie missing director".to_string(),
                    description: "Movie schema is missing the director property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add director to Movie structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingAuthorValidatorV2;
impl Default for BookMissingAuthorValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingAuthorValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingAuthorValidatorV2 {
    fn name(&self) -> &str {
        "book-missing-author-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd.data.get("author").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKAUTH-V2001".to_string(),
                    title: "Book missing author".to_string(),
                    description: "Book schema is missing the author property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add author to Book structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingIsbnValidatorV2;
impl Default for BookMissingIsbnValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingIsbnValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingIsbnValidatorV2 {
    fn name(&self) -> &str {
        "book-missing-isbn-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd.data.get("isbn").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKISBN-V2001".to_string(),
                    title: "Book missing ISBN".to_string(),
                    description: "Book schema is missing the isbn property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add isbn to Book structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct BookMissingDatePublishedValidatorV2;
impl Default for BookMissingDatePublishedValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl BookMissingDatePublishedValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for BookMissingDatePublishedValidatorV2 {
    fn name(&self) -> &str {
        "book-missing-date-published-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" {
                continue;
            }
            if sd.data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BOOKDATE-V2001".to_string(),
                    title: "Book missing datePublished".to_string(),
                    description: "Book schema is missing the datePublished property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add datePublished to Book structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ServiceMissingProviderValidatorV2;
impl Default for ServiceMissingProviderValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ServiceMissingProviderValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ServiceMissingProviderValidatorV2 {
    fn name(&self) -> &str {
        "service-missing-provider-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" {
                continue;
            }
            if sd.data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SVCPRV-V2001".to_string(),
                    title: "Service missing provider".to_string(),
                    description: "Service schema is missing the provider property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add provider to Service structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HealthPlanMissingProviderValidatorV2;
impl Default for HealthPlanMissingProviderValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl HealthPlanMissingProviderValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HealthPlanMissingProviderValidatorV2 {
    fn name(&self) -> &str {
        "health-plan-missing-provider-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HealthPlan" {
                continue;
            }
            if sd.data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "HPPRV-V2001".to_string(),
                    title: "HealthPlan missing provider".to_string(),
                    description: "HealthPlan schema is missing the provider property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add provider to HealthPlan structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct InvoiceMissingAccountValidatorV2;
impl Default for InvoiceMissingAccountValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl InvoiceMissingAccountValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for InvoiceMissingAccountValidatorV2 {
    fn name(&self) -> &str {
        "invoice-missing-account-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Invoice" {
                continue;
            }
            if sd.data.get("accountId").is_none() && sd.data.get("account").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "INVACCT-V2001".to_string(),
                    title: "Invoice missing account".to_string(),
                    description: "Invoice schema has no accountId or account property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add accountId or account to Invoice structured data."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermitMissingPermitNumberValidatorV2;
impl Default for PermitMissingPermitNumberValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl PermitMissingPermitNumberValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermitMissingPermitNumberValidatorV2 {
    fn name(&self) -> &str {
        "permit-missing-permit-number-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Permit" {
                continue;
            }
            if sd.data.get("permitNumber").is_none() && sd.data.get("identifier").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERMNUM-V2001".to_string(),
                    title: "Permit missing permitNumber".to_string(),
                    description: "Permit schema has no permitNumber or identifier property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add permitNumber or identifier to Permit structured data."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct PlanMissingDescriptionValidatorV2;
impl Default for PlanMissingDescriptionValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl PlanMissingDescriptionValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PlanMissingDescriptionValidatorV2 {
    fn name(&self) -> &str {
        "plan-missing-description-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Plan" {
                continue;
            }
            if sd.data.get("description").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLANDESC-V2001".to_string(),
                    title: "Plan missing description".to_string(),
                    description: "Plan schema is missing the description property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add description to Plan structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ResearchProjectMissingAboutValidatorV2;
impl Default for ResearchProjectMissingAboutValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ResearchProjectMissingAboutValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ResearchProjectMissingAboutValidatorV2 {
    fn name(&self) -> &str {
        "research-project-missing-about-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "ResearchProject" {
                continue;
            }
            if sd.data.get("about").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RPABOUT-V2001".to_string(),
                    title: "ResearchProject missing about".to_string(),
                    description: "ResearchProject schema is missing the about property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add about to ResearchProject structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ScheduleMissingTimezoneValidatorV2;
impl Default for ScheduleMissingTimezoneValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ScheduleMissingTimezoneValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ScheduleMissingTimezoneValidatorV2 {
    fn name(&self) -> &str {
        "schedule-missing-timezone-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Schedule" {
                continue;
            }
            if sd.data.get("timezone").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "SCHEDTZ-V2001".to_string(),
                    title: "Schedule missing timezone".to_string(),
                    description: "Schedule schema is missing the timezone property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add timezone to Schedule structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TripMissingItineraryValidatorV2;
impl Default for TripMissingItineraryValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TripMissingItineraryValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TripMissingItineraryValidatorV2 {
    fn name(&self) -> &str {
        "trip-missing-itinerary-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Trip" {
                continue;
            }
            if sd.data.get("itinerary").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TRIPIT-V2001".to_string(),
                    title: "Trip missing itinerary".to_string(),
                    description: "Trip schema is missing the itinerary property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add itinerary to Trip structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WorkersUnionMissingNameValidatorV2;
impl Default for WorkersUnionMissingNameValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkersUnionMissingNameValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WorkersUnionMissingNameValidatorV2 {
    fn name(&self) -> &str {
        "workers-union-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WorkersUnion" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WUNAME-V2001".to_string(),
                    title: "WorkersUnion missing name".to_string(),
                    description: "WorkersUnion schema is missing or has empty name.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to WorkersUnion structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WebAPIMissingDocumentationValidatorV2;
impl Default for WebAPIMissingDocumentationValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl WebAPIMissingDocumentationValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WebAPIMissingDocumentationValidatorV2 {
    fn name(&self) -> &str {
        "webapi-missing-documentation-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebAPI" {
                continue;
            }
            if sd.data.get("documentation").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WAPIDOC-V2001".to_string(),
                    title: "WebAPI missing documentation".to_string(),
                    description: "WebAPI schema is missing the documentation property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add documentation to WebAPI structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WearableMissingDeviceTypeValidatorV2;
impl Default for WearableMissingDeviceTypeValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl WearableMissingDeviceTypeValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WearableMissingDeviceTypeValidatorV2 {
    fn name(&self) -> &str {
        "wearable-missing-device-type-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Wearable" {
                continue;
            }
            if sd.data.get("deviceType").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WEARDEV-V2001".to_string(),
                    title: "Wearable missing deviceType".to_string(),
                    description: "Wearable schema is missing the deviceType property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add deviceType to Wearable structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WebPageElementMissingNameValidatorV2;
impl Default for WebPageElementMissingNameValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl WebPageElementMissingNameValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WebPageElementMissingNameValidatorV2 {
    fn name(&self) -> &str {
        "webpage-element-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebPageElement" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WPELNAME-V2001".to_string(),
                    title: "WebPageElement missing name".to_string(),
                    description: "WebPageElement schema is missing or has empty name.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to WebPageElement structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct WorkerMissingJobTitleValidatorV2;
impl Default for WorkerMissingJobTitleValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl WorkerMissingJobTitleValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for WorkerMissingJobTitleValidatorV2 {
    fn name(&self) -> &str {
        "worker-missing-job-title-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Worker" {
                continue;
            }
            if sd.data.get("jobTitle").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WORKJOB-V2001".to_string(),
                    title: "Worker missing jobTitle".to_string(),
                    description: "Worker schema is missing the jobTitle property.".to_string(),
                    url: url.clone(),
                    recommendation: "Add jobTitle to Worker structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct OrganizationMissingNameValidatorV2;
impl Default for OrganizationMissingNameValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl OrganizationMissingNameValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for OrganizationMissingNameValidatorV2 {
    fn name(&self) -> &str {
        "organization-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Organization" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ORGNAME-V2001".to_string(),
                    title: "Organization missing name".to_string(),
                    description: "Organization schema is missing or has empty name.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to Organization structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PersonMissingNameValidatorV2;
impl Default for PersonMissingNameValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl PersonMissingNameValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PersonMissingNameValidatorV2 {
    fn name(&self) -> &str {
        "person-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Person" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PERSNAME-V2001".to_string(),
                    title: "Person missing name".to_string(),
                    description: "Person schema is missing or has empty name.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to Person structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct JobPostingMissingTitleValidatorV2;
impl Default for JobPostingMissingTitleValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl JobPostingMissingTitleValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for JobPostingMissingTitleValidatorV2 {
    fn name(&self) -> &str {
        "job-posting-missing-title-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "JobPosting" {
                continue;
            }
            if sd
                .data
                .get("title")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Schema,
                    code: "JOBTITLE-V2001".to_string(),
                    title: "JobPosting missing title".to_string(),
                    description: "JobPosting schema is missing or has empty title.".to_string(),
                    url: url.clone(),
                    recommendation: "Add title to JobPosting structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CourseMissingNameValidatorV2;
impl Default for CourseMissingNameValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CourseMissingNameValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CourseMissingNameValidatorV2 {
    fn name(&self) -> &str {
        "course-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Course" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "COURSENAME-V2001".to_string(),
                    title: "Course missing name".to_string(),
                    description: "Course schema is missing or has empty name.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to Course structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct RecipeMissingNameValidatorV2;
impl Default for RecipeMissingNameValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl RecipeMissingNameValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for RecipeMissingNameValidatorV2 {
    fn name(&self) -> &str {
        "recipe-missing-name-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Recipe" {
                continue;
            }
            if sd
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RECIPENAME-V2001".to_string(),
                    title: "Recipe missing name".to_string(),
                    description: "Recipe schema is missing or has empty name.".to_string(),
                    url: url.clone(),
                    recommendation: "Add name to Recipe structured data.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LocalBusinessMissingNpiValidatorV2;
impl Default for LocalBusinessMissingNpiValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl LocalBusinessMissingNpiValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LocalBusinessMissingNpiValidatorV2 {
    fn name(&self) -> &str {
        "local-business-missing-npi-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LocalBusiness" {
                continue;
            }
            if sd.data.get("identifier").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "LBPI-V2001".to_string(),
                    title: "LocalBusiness missing identifier (NPI)".to_string(),
                    description: "LocalBusiness schema is missing the identifier property for NPI."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add identifier (NPI) to LocalBusiness structured data."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// V8 Security Validators (25)
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::StructuredData;

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
    fn test_org_url_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"name": "Acme"}),
        }];
        let f = OrganizationUrlValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_org_url_present() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"url": "https://example.com"}),
        }];
        assert!(OrganizationUrlValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_org_logo_url_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"logo": {"@type": "ImageObject"}}),
        }];
        let f = OrganizationLogoUrlValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_org_logo_string() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"logo": "https://example.com/logo.png"}),
        }];
        assert!(OrganizationLogoUrlValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_org_contact_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({}),
        }];
        let f = OrganizationContactValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_person_url_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Person".into()),
            data: serde_json::json!({"name": "Jane"}),
        }];
        let f = PersonUrlValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_person_url_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Person".into()),
            data: serde_json::json!({"url": "https://example.com/jane"}),
        }];
        assert!(PersonUrlValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_job_employment_type_invalid() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("JobPosting".into()),
            data: serde_json::json!({"employmentType": "FULL"}),
        }];
        let f = JobPostingEmploymentTypeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_job_employment_type_valid() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("JobPosting".into()),
            data: serde_json::json!({"employmentType": "FULL_TIME"}),
        }];
        assert!(JobPostingEmploymentTypeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_job_salary_currency_invalid() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("JobPosting".into()),
            data: serde_json::json!({"baseSalary": {"currency": "US"}}),
        }];
        let f = JobPostingSalaryCurrencyValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_job_valid_through_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("JobPosting".into()),
            data: serde_json::json!({}),
        }];
        let f = JobPostingValidThroughValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_course_desc_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Course".into()),
            data: serde_json::json!({}),
        }];
        let f = CourseDescriptionValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_course_provider_name_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Course".into()),
            data: serde_json::json!({"provider": {"@type": "Organization"}}),
        }];
        let f = CourseProviderNameValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_recipe_prep_time_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({}),
        }];
        let f = RecipePrepTimeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_recipe_ingredients_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({}),
        }];
        let f = RecipeIngredientsValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_recipe_ingredients_empty() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"recipeIngredient": []}),
        }];
        let f = RecipeIngredientsValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_product_price_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({"offers": {"@type": "Offer"}}),
        }];
        let f = ProductPriceValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_product_image_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({}),
        }];
        let f = ProductImageValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_breadcrumb_item_count() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("BreadcrumbList".into()),
            data: serde_json::json!({"itemListElement": [{"@type": "ListItem"}]}),
        }];
        let f = BreadcrumbItemCountValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_breadcrumb_url_relative() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("BreadcrumbList".into()),
            data: serde_json::json!({"itemListElement": [{"@type": "ListItem", "item": {"@id": "/about"}}]}),
        }];
        let f = BreadcrumbUrlValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_event_organizer_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({}),
        }];
        let f = EventOrganizerValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_event_organizer_no_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({"organizer": {"@type": "Organization"}}),
        }];
        let f = EventOrganizerValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_event_performer_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({}),
        }];
        let f = EventPerformerValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_video_thumbnail_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("VideoObject".into()),
            data: serde_json::json!({}),
        }];
        let f = VideoThumbnailValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_video_duration_format() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("VideoObject".into()),
            data: serde_json::json!({"duration": "5min"}),
        }];
        let f = VideoDurationFormatValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_video_duration_iso() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("VideoObject".into()),
            data: serde_json::json!({"duration": "PT5M"}),
        }];
        assert!(VideoDurationFormatValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_software_offers_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("SoftwareApplication".into()),
            data: serde_json::json!({}),
        }];
        let f = SoftwareOffersValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_software_screenshot_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("SoftwareApplication".into()),
            data: serde_json::json!({}),
        }];
        let f = SoftwareScreenshotValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_faq_answer_short() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("FAQPage".into()),
            data: serde_json::json!({"mainEntity": [{"@type": "Question", "acceptedAnswer": {"@type": "Answer", "text": "Short"}}]}),
        }];
        let f = FAQAnswerLengthValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_faq_answer_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("FAQPage".into()),
            data: serde_json::json!({"mainEntity": [{"@type": "Question", "acceptedAnswer": {"@type": "Answer", "text": "This is a much longer answer that should be well over the fifty character minimum threshold."}}]}),
        }];
        assert!(FAQAnswerLengthValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_name_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({}),
        }];
        let f = HowToNameValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_howto_step_desc_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"step": [{"@type": "HowToStep"}]}),
        }];
        let f = HowToStepDescriptionValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_dataset_license_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({}),
        }];
        let f = DatasetLicenseValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_dataset_dist_no_url() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"distribution": [{"@type": "DataDownload"}]}),
        }];
        let f = DatasetDistributionValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }

    // ===== Security V5 Tests =====
    #[test]
    fn test_creative_work_name_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("CreativeWork".into()),
            data: serde_json::json!({}),
        }];
        let f = CreativeWorkMissingNameValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CWNAME-V6001");
    }
    #[test]
    fn test_creative_work_name_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("CreativeWork".into()),
            data: serde_json::json!({"name": "My Work"}),
        }];
        assert!(CreativeWorkMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_creative_work_desc_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("CreativeWork".into()),
            data: serde_json::json!({}),
        }];
        assert!(!CreativeWorkMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_creative_work_date_missing() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("CreativeWork".into()),
            data: serde_json::json!({}),
        }];
        assert!(!CreativeWorkMissingDateCreatedValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_playlist_num_items() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Playlist".into()),
            data: serde_json::json!({}),
        }];
        assert!(!PlaylistMissingNumberOfItemsValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_food_est_menu() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("FoodEstablishment".into()),
            data: serde_json::json!({}),
        }];
        assert!(!FoodEstablishmentMissingMenuValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_food_est_cuisine() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("FoodEstablishment".into()),
            data: serde_json::json!({}),
        }];
        assert!(!FoodEstablishmentMissingServesCuisineValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_lodging_star_rating() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LodgingBusiness".into()),
            data: serde_json::json!({}),
        }];
        assert!(!LodgingBusinessMissingStarRatingValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_lodging_amenity() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LodgingBusiness".into()),
            data: serde_json::json!({}),
        }];
        assert!(!LodgingBusinessMissingAmenityFeatureValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sports_loc_sport() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("SportsActivityLocation".into()),
            data: serde_json::json!({}),
        }];
        assert!(!SportsActivityLocationMissingSportValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_civic_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("CivicStructure".into()),
            data: serde_json::json!({}),
        }];
        assert!(!CivicStructureMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_landform_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Landform".into()),
            data: serde_json::json!({}),
        }];
        assert!(!LandformMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tourist_attr_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TouristAttraction".into()),
            data: serde_json::json!({}),
        }];
        assert!(!TouristAttractionMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tourist_dest_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TouristDestination".into()),
            data: serde_json::json!({}),
        }];
        assert!(!TouristDestinationMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sports_event_sport() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("SportsEvent".into()),
            data: serde_json::json!({}),
        }];
        assert!(!SportsEventMissingSportValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sports_event_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("SportsEvent".into()),
            data: serde_json::json!({}),
        }];
        assert!(!SportsEventMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_edu_org_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("EducationalOrganization".into()),
            data: serde_json::json!({}),
        }];
        assert!(!EducationalOrganizationMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ngo_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("NGO".into()),
            data: serde_json::json!({}),
        }];
        assert!(!NGOMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_performing_arts_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("PerformingArtsSeries".into()),
            data: serde_json::json!({}),
        }];
        assert!(!PerformingArtsSeriesMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_broadcast_event_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("BroadcastEvent".into()),
            data: serde_json::json!({}),
        }];
        assert!(!BroadcastEventMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_product_brand() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ProductMissingBrandValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_product_category() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ProductMissingCategoryValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_product_review() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ProductMissingReviewValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_author() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({}),
        }];
        assert!(!BookMissingAuthorValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_isbn() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({}),
        }];
        assert!(!BookMissingIsbnValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_date_published() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({}),
        }];
        assert!(!BookMissingDatePublishedValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_director() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({}),
        }];
        assert!(!MovieMissingDirectorValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_duration() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({}),
        }];
        assert!(!MovieMissingDurationValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_date_created() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({}),
        }];
        assert!(!MovieMissingDateCreatedValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tv_series_seasons() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({}),
        }];
        assert!(!TVSeriesMissingNumberOfSeasonsValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tv_series_episode() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({}),
        }];
        assert!(!TVSeriesMissingEpisodeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_music_recording_artist() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicRecording".into()),
            data: serde_json::json!({}),
        }];
        assert!(!MusicRecordingMissingByArtistValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_music_recording_album() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicRecording".into()),
            data: serde_json::json!({}),
        }];
        assert!(!MusicRecordingMissingAlbumValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_service_area_served() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Service".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ServiceMissingAreaServedValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_service_provider() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Service".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ServiceMissingProviderValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_health_plan_provider() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HealthPlan".into()),
            data: serde_json::json!({}),
        }];
        assert!(!HealthPlanMissingProviderValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_health_plan_coverage() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HealthPlan".into()),
            data: serde_json::json!({}),
        }];
        assert!(!HealthPlanMissingCoverageAreaValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_invoice_account() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Invoice".into()),
            data: serde_json::json!({}),
        }];
        assert!(!InvoiceMissingAccountValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_invoice_payment_date() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Invoice".into()),
            data: serde_json::json!({}),
        }];
        assert!(!InvoiceMissingPaymentDueDateValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_permit_number() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Permit".into()),
            data: serde_json::json!({}),
        }];
        assert!(!PermitMissingPermitNumberValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_permit_issued_by() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Permit".into()),
            data: serde_json::json!({}),
        }];
        assert!(!PermitMissingIssuedByValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_plan_description() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Plan".into()),
            data: serde_json::json!({}),
        }];
        assert!(!PlanMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_plan_about() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Plan".into()),
            data: serde_json::json!({}),
        }];
        assert!(!PlanMissingAboutValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_research_about() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("ResearchProject".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ResearchProjectMissingAboutValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_research_funder() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("ResearchProject".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ResearchProjectMissingFunderValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_schedule_tz() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Schedule".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ScheduleMissingTimezoneValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_trip_itinerary() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Trip".into()),
            data: serde_json::json!({}),
        }];
        assert!(!TripMissingItineraryValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_workers_union_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WorkersUnion".into()),
            data: serde_json::json!({}),
        }];
        assert!(!WorkersUnionMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_webapi_doc() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebAPI".into()),
            data: serde_json::json!({}),
        }];
        assert!(!WebAPIMissingDocumentationValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_wearable_device() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Wearable".into()),
            data: serde_json::json!({}),
        }];
        assert!(!WearableMissingDeviceTypeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_webpage_element_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebPageElement".into()),
            data: serde_json::json!({}),
        }];
        assert!(!WebPageElementMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_worker_job_title() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Worker".into()),
            data: serde_json::json!({}),
        }];
        assert!(!WorkerMissingJobTitleValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_creative_work_license() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("CreativeWork".into()),
            data: serde_json::json!({}),
        }];
        assert!(!CreativeWorkMissingLicenseValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_product_sku() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ProductMissingSkuValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_publisher() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({}),
        }];
        assert!(!BookMissingPublisherValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_actor() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({}),
        }];
        assert!(!MovieMissingActorValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_service_type() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Service".into()),
            data: serde_json::json!({}),
        }];
        assert!(!ServiceMissingServiceTypeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_event_start_date() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({}),
        }];
        assert!(!EventMissingStartDateValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_event_location() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({}),
        }];
        assert!(!EventMissingLocationValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_local_business_geo() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LocalBusiness".into()),
            data: serde_json::json!({}),
        }];
        assert!(!LocalBusinessMissingGeoValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V6 Security Validators Tests =====
    #[test]
    fn test_apt_missing_rooms() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({}),
        }];
        let f = ApartmentMissingNumberOfRoomsValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "APTROOM001");
    }
    #[test]
    fn test_apt_rooms_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"numberOfRooms": 3}),
        }];
        assert!(ApartmentMissingNumberOfRoomsValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_car_missing_model() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({}),
        }];
        let f = CarMissingModelValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CARMODEL001");
    }
    #[test]
    fn test_car_model_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({"model": "Civic"}),
        }];
        assert!(CarMissingModelValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_car_missing_manufacturer() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({}),
        }];
        let f = CarMissingManufacturerValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CARMFR001");
    }
    #[test]
    fn test_car_manufacturer_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({"manufacturer": "Honda"}),
        }];
        assert!(CarMissingManufacturerValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_musicalbum_missing_tracks() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicAlbum".into()),
            data: serde_json::json!({}),
        }];
        let f = MusicAlbumMissingNumberOfTracksValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "MUSTRACKS001");
    }
    #[test]
    fn test_musicalbum_tracks_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicAlbum".into()),
            data: serde_json::json!({"numberOfTracks": 12}),
        }];
        assert!(MusicAlbumMissingNumberOfTracksValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tvseries_missing_episodes() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({}),
        }];
        let f = TVSeriesMissingNumberOfEpisodesValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TVEPISODES001");
    }
    #[test]
    fn test_tvseries_episodes_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({"numberOfEpisodes": 24}),
        }];
        assert!(TVSeriesMissingNumberOfEpisodesValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_missing_rating() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({}),
        }];
        let f = MovieMissingAggregateRatingValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "MOVRATING001");
    }
    #[test]
    fn test_movie_rating_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({"aggregateRating": {"ratingValue": 8.5}}),
        }];
        assert!(MovieMissingAggregateRatingValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_gov_service_missing_area() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("GovernmentService".into()),
            data: serde_json::json!({}),
        }];
        let f = GovernmentServiceMissingServiceAreaValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "GOVSERVICE001");
    }
    #[test]
    fn test_gov_service_area_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("GovernmentService".into()),
            data: serde_json::json!({"serviceArea": "NYC"}),
        }];
        assert!(GovernmentServiceMissingServiceAreaValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_healthplan_missing_id() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HealthPlan".into()),
            data: serde_json::json!({}),
        }];
        let f = HealthPlanMissingHealthPlanIdValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HPID001");
    }
    #[test]
    fn test_healthplan_id_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HealthPlan".into()),
            data: serde_json::json!({"healthPlanId": "HP-001"}),
        }];
        assert!(HealthPlanMissingHealthPlanIdValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_invoice_missing_status() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Invoice".into()),
            data: serde_json::json!({}),
        }];
        let f = InvoiceMissingPaymentStatusValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "INVSTATUS001");
    }
    #[test]
    fn test_invoice_status_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Invoice".into()),
            data: serde_json::json!({"paymentStatus": "Paid"}),
        }];
        assert!(InvoiceMissingPaymentStatusValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_permit_missing_validfrom() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Permit".into()),
            data: serde_json::json!({}),
        }];
        let f = PermitMissingValidFromValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PERMFROM001");
    }
    #[test]
    fn test_permit_validfrom_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Permit".into()),
            data: serde_json::json!({"validFrom": "2024-01-01"}),
        }];
        assert!(PermitMissingValidFromValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_plan_missing_benefits() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Plan".into()),
            data: serde_json::json!({}),
        }];
        let f = PlanMissingBenefitsValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PLANBEN001");
    }
    #[test]
    fn test_plan_benefits_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Plan".into()),
            data: serde_json::json!({"benefits": "Health coverage"}),
        }];
        assert!(PlanMissingBenefitsValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_rp_missing_funder() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("ResearchProject".into()),
            data: serde_json::json!({}),
        }];
        let f =
            ResearchProjectMissingFundingRecognizerValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "RPFUND001");
    }
    #[test]
    fn test_rp_funder_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("ResearchProject".into()),
            data: serde_json::json!({"funder": "NSF"}),
        }];
        assert!(ResearchProjectMissingFundingRecognizerValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_schedule_missing_freq() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Schedule".into()),
            data: serde_json::json!({}),
        }];
        let f = ScheduleMissingRepeatFrequencyValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "SCHEDFREQ001");
    }
    #[test]
    fn test_schedule_freq_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Schedule".into()),
            data: serde_json::json!({"repeatFrequency": "P1W"}),
        }];
        assert!(ScheduleMissingRepeatFrequencyValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_trip_missing_departure() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Trip".into()),
            data: serde_json::json!({}),
        }];
        let f = TripMissingDepartureTimeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TRIPDEP001");
    }
    #[test]
    fn test_trip_departure_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Trip".into()),
            data: serde_json::json!({"departureTime": "2024-06-01T10:00:00"}),
        }];
        assert!(TripMissingDepartureTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_wu_missing_count() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WorkersUnion".into()),
            data: serde_json::json!({}),
        }];
        let f = WorkersUnionMissingMemberCountValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WUMEMBER001");
    }
    #[test]
    fn test_wu_count_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WorkersUnion".into()),
            data: serde_json::json!({"memberCount": 500}),
        }];
        assert!(WorkersUnionMissingMemberCountValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_webapi_missing_docs() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebAPI".into()),
            data: serde_json::json!({}),
        }];
        let f = WebAPIDocumentationMissingValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WAPIDOCS001");
    }
    #[test]
    fn test_webapi_docs_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebAPI".into()),
            data: serde_json::json!({"documentation": "https://docs.example.com"}),
        }];
        assert!(WebAPIDocumentationMissingValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_wearable_missing_battery() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Wearable".into()),
            data: serde_json::json!({}),
        }];
        let f = WearableMissingBatteryLifeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WEARBATT001");
    }
    #[test]
    fn test_wearable_battery_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Wearable".into()),
            data: serde_json::json!({"batteryLife": "24h"}),
        }];
        assert!(WearableMissingBatteryLifeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_webpage_element_missing_name() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebPageElement".into()),
            data: serde_json::json!({}),
        }];
        let f = WebPageElementMissingAccessibleNameValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WPELACC001");
    }
    #[test]
    fn test_webpage_element_name_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebPageElement".into()),
            data: serde_json::json!({"name": "Header"}),
        }];
        assert!(WebPageElementMissingAccessibleNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_worker_missing_occupation() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Worker".into()),
            data: serde_json::json!({}),
        }];
        let f = WorkerMissingOccupationValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WOCCUP001");
    }
    #[test]
    fn test_worker_occupation_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Worker".into()),
            data: serde_json::json!({"occupation": "Engineer"}),
        }];
        assert!(WorkerMissingOccupationValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_missing_format() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({}),
        }];
        let f = BookMissingBookFormatValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "BOOKFMT001");
    }
    #[test]
    fn test_book_format_ok() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"bookFormat": "Hardcover"}),
        }];
        assert!(BookMissingBookFormatValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V7 Security Validators Tests =====
    #[test]
    fn test_dataset_missing_description() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather"}),
        }];
        let f = DatasetMissingDescriptionValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "DATDESC001");
    }
    #[test]
    fn test_dataset_with_description() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "description": "Daily weather data"}),
        }];
        assert!(DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_empty_description() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "description": ""}),
        }];
        assert!(!DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_non_dataset_ignored() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        assert!(DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_no_structured_data() {
        let p = make_page("https://example.com");
        assert!(DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_multiple_missing() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Dataset".into()),
                data: serde_json::json!({"@type": "Dataset"}),
            },
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Dataset".into()),
                data: serde_json::json!({"@type": "Dataset"}),
            },
        ];
        let f = DatasetMissingDescriptionValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_dataset_name_whitespace_description() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "description": "   "}),
        }];
        assert!(!DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_description_null() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "description": null}),
        }];
        assert!(!DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_description_number() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "description": 123}),
        }];
        assert!(DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_description_array() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "description": []}),
        }];
        assert!(DatasetMissingDescriptionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== DatasetMissingDistributionValidator tests =====
    #[test]
    fn test_dataset_missing_distribution() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather"}),
        }];
        let f = DatasetMissingDistributionValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "DATDIST001");
    }
    #[test]
    fn test_dataset_with_distribution() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "distribution": [{"@type": "DataDownload", "contentUrl": "https://example.com/data.csv"}]}),
        }];
        assert!(DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_empty_distribution() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "distribution": []}),
        }];
        assert!(!DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_non_dataset_ignored_dist() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Article".into()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        assert!(DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_no_structured_data_dist() {
        let p = make_page("https://example.com");
        assert!(DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_multiple_missing_dist() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Dataset".into()),
                data: serde_json::json!({"@type": "Dataset"}),
            },
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Dataset".into()),
                data: serde_json::json!({"@type": "Dataset"}),
            },
        ];
        let f = DatasetMissingDistributionValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_dataset_distribution_string() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "distribution": "https://example.com/data.csv"}),
        }];
        assert!(DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_distribution_null() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "distribution": null}),
        }];
        assert!(!DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_dataset_distribution_number() {
        let mut p = make_page("https://example.com/data");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Dataset".into()),
            data: serde_json::json!({"@type": "Dataset", "name": "Weather", "distribution": 42}),
        }];
        assert!(DatasetMissingDistributionValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== HowToMissingNameValidator tests =====
    #[test]
    fn test_howto_missing_name_v8() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "step": []}),
        }];
        let f = HowToMissingNameValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HOWNAME001");
    }
    #[test]
    fn test_howto_with_name() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to bake a cake"}),
        }];
        assert!(HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_empty_name() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "name": ""}),
        }];
        assert!(!HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_non_howto_ignored() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({"@type": "Event"}),
        }];
        assert!(HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_no_structured_data_name() {
        let p = make_page("https://example.com");
        assert!(HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_multiple_missing_name() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("HowTo".into()),
                data: serde_json::json!({"@type": "HowTo"}),
            },
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("HowTo".into()),
                data: serde_json::json!({"@type": "HowTo"}),
            },
        ];
        let f = HowToMissingNameValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_howto_name_whitespace() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "name": "   "}),
        }];
        assert!(!HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_name_null() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "name": null}),
        }];
        assert!(!HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_name_number() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "name": 123}),
        }];
        assert!(HowToMissingNameValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== HowToMissingStepValidator tests =====
    #[test]
    fn test_howto_missing_step_v8() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "name": "How to bake"}),
        }];
        let f = HowToMissingStepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HOWSTEP001");
    }
    #[test]
    fn test_howto_with_step() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "step": [{"@type": "HowToStep", "name": "Step 1"}]}),
        }];
        assert!(HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_empty_step_array() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "step": []}),
        }];
        assert!(!HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_non_howto_ignored_step() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Event".into()),
            data: serde_json::json!({"@type": "Event"}),
        }];
        assert!(HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_no_structured_data_step() {
        let p = make_page("https://example.com");
        assert!(HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_multiple_missing_step() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("HowTo".into()),
                data: serde_json::json!({"@type": "HowTo"}),
            },
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("HowTo".into()),
                data: serde_json::json!({"@type": "HowTo"}),
            },
        ];
        let f = HowToMissingStepValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_howto_step_string() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "step": "Step 1"}),
        }];
        assert!(HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_step_number() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "step": 3}),
        }];
        assert!(HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_howto_step_object() {
        let mut p = make_page("https://example.com/howto");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HowTo".into()),
            data: serde_json::json!({"@type": "HowTo", "step": {"@type": "HowToStep", "name": "Step 1"}}),
        }];
        assert!(HowToMissingStepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== RecipeMissingCookTimeValidator tests =====
    #[test]
    fn test_recipe_missing_cooktime() {
        let mut p = make_page("https://example.com/recipe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta"}),
        }];
        let f = RecipeMissingCookTimeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "RECCOOK001");
    }
    #[test]
    fn test_recipe_with_cooktime() {
        let mut p = make_page("https://example.com/recipe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "cookTime": "PT30M"}),
        }];
        assert!(RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_empty_cooktime() {
        let mut p = make_page("https://example.com/recipe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "cookTime": ""}),
        }];
        assert!(!RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_non_recipe_ignored() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        assert!(RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_no_structured_data() {
        let p = make_page("https://example.com");
        assert!(RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_multiple_missing_cooktime() {
        let mut p = make_page("https://example.com/recipes");
        p.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Recipe".into()),
                data: serde_json::json!({"@type": "Recipe", "name": "Pasta"}),
            },
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Recipe".into()),
                data: serde_json::json!({"@type": "Recipe", "name": "Soup"}),
            },
        ];
        let f = RecipeMissingCookTimeValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_recipe_cooktime_whitespace() {
        let mut p = make_page("https://example.com/recipe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "cookTime": "   "}),
        }];
        assert!(!RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_cooktime_number() {
        let mut p = make_page("https://example.com/recipe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "cookTime": 1800}),
        }];
        assert!(RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_cooktime_null() {
        let mut p = make_page("https://example.com/recipe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "cookTime": null}),
        }];
        assert!(!RecipeMissingCookTimeValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== PermissionsPolicyPaymentDeepValidator tests =====
    #[test]
    fn test_apt_room_v8_missing() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment"}),
        }];
        let f = ApartmentMissingNumberOfRoomsValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "APTROOM-V2001");
    }
    #[test]
    fn test_apt_room_v8_present() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment", "numberOfRooms": 3}),
        }];
        assert!(ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_empty() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment", "numberOfRooms": ""}),
        }];
        assert!(!ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_null() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment", "numberOfRooms": null}),
        }];
        assert!(!ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_non_apt() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Hotel".into()),
            data: serde_json::json!({"@type": "Hotel"}),
        }];
        assert!(ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_no_sd() {
        let p = make_page("https://example.com");
        assert!(ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_number() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment", "numberOfRooms": 2}),
        }];
        assert!(ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_multiple() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Apartment".into()),
                data: serde_json::json!({"@type": "Apartment"}),
            },
            StructuredData {
                context: Some("https://schema.org".into()),
                r#type: Some("Apartment".into()),
                data: serde_json::json!({"@type": "Apartment"}),
            },
        ];
        let f = ApartmentMissingNumberOfRoomsValidatorV2::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_apt_room_v8_whitespace() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment", "numberOfRooms": "   "}),
        }];
        assert!(!ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_apt_room_v8_zero() {
        let mut p = make_page("https://example.com/apt");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Apartment".into()),
            data: serde_json::json!({"@type": "Apartment", "numberOfRooms": 0}),
        }];
        assert!(ApartmentMissingNumberOfRoomsValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_car_model_v8_missing() {
        let mut p = make_page("https://example.com/car");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({"@type": "Car"}),
        }];
        let f = CarMissingModelValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CARMODEL-V2001");
    }
    #[test]
    fn test_car_model_v8_present() {
        let mut p = make_page("https://example.com/car");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({"@type": "Car", "model": "Model 3"}),
        }];
        assert!(CarMissingModelValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_car_model_v8_empty() {
        let mut p = make_page("https://example.com/car");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Car".into()),
            data: serde_json::json!({"@type": "Car", "model": ""}),
        }];
        assert!(!CarMissingModelValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_car_model_v8_non_car() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Product".into()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        assert!(CarMissingModelValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_car_model_v8_no_sd() {
        let p = make_page("https://example.com");
        assert!(CarMissingModelValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_music_tracks_v8_missing() {
        let mut p = make_page("https://example.com/album");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicAlbum".into()),
            data: serde_json::json!({"@type": "MusicAlbum"}),
        }];
        let f = MusicAlbumMissingTracksValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "MUSCTRACKS-V2001");
    }
    #[test]
    fn test_music_tracks_v8_with_track() {
        let mut p = make_page("https://example.com/album");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicAlbum".into()),
            data: serde_json::json!({"@type": "MusicAlbum", "track": [{"@type": "MusicRecording"}]}),
        }];
        assert!(MusicAlbumMissingTracksValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_music_tracks_v8_with_numtracks() {
        let mut p = make_page("https://example.com/album");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("MusicAlbum".into()),
            data: serde_json::json!({"@type": "MusicAlbum", "numTracks": 12}),
        }];
        assert!(MusicAlbumMissingTracksValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_music_tracks_v8_no_sd() {
        let p = make_page("https://example.com");
        assert!(MusicAlbumMissingTracksValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tv_episodes_v8_missing() {
        let mut p = make_page("https://example.com/tv");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({"@type": "TVSeries"}),
        }];
        let f = TVSeriesMissingEpisodesValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TVEP-V2001");
    }
    #[test]
    fn test_tv_episodes_v8_with_episode() {
        let mut p = make_page("https://example.com/tv");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({"@type": "TVSeries", "episode": [{"@type": "TVEpisode"}]}),
        }];
        assert!(TVSeriesMissingEpisodesValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tv_episodes_v8_with_count() {
        let mut p = make_page("https://example.com/tv");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("TVSeries".into()),
            data: serde_json::json!({"@type": "TVSeries", "numberOfEpisodes": 24}),
        }];
        assert!(TVSeriesMissingEpisodesValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tv_episodes_v8_no_sd() {
        let p = make_page("https://example.com");
        assert!(TVSeriesMissingEpisodesValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_director_v8_missing() {
        let mut p = make_page("https://example.com/movie");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({"@type": "Movie"}),
        }];
        let f = MovieMissingDirectorValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "MOVDIR-V2001");
    }
    #[test]
    fn test_movie_director_v8_present() {
        let mut p = make_page("https://example.com/movie");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Movie".into()),
            data: serde_json::json!({"@type": "Movie", "director": {"@type": "Person", "name": "Spielberg"}}),
        }];
        assert!(MovieMissingDirectorValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_movie_director_v8_no_sd() {
        let p = make_page("https://example.com");
        assert!(MovieMissingDirectorValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_author_v8_missing() {
        let mut p = make_page("https://example.com/book");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"@type": "Book"}),
        }];
        let f = BookMissingAuthorValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "BOOKAUTH-V2001");
    }
    #[test]
    fn test_book_author_v8_present() {
        let mut p = make_page("https://example.com/book");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"@type": "Book", "author": "Author"}),
        }];
        assert!(BookMissingAuthorValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_isbn_v8_missing() {
        let mut p = make_page("https://example.com/book");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"@type": "Book"}),
        }];
        let f = BookMissingIsbnValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "BOOKISBN-V2001");
    }
    #[test]
    fn test_book_isbn_v8_present() {
        let mut p = make_page("https://example.com/book");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"@type": "Book", "isbn": "978-3-16-148410-0"}),
        }];
        assert!(BookMissingIsbnValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_book_date_v8_missing() {
        let mut p = make_page("https://example.com/book");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"@type": "Book"}),
        }];
        let f = BookMissingDatePublishedValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "BOOKDATE-V2001");
    }
    #[test]
    fn test_book_date_v8_present() {
        let mut p = make_page("https://example.com/book");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Book".into()),
            data: serde_json::json!({"@type": "Book", "datePublished": "2023-01-01"}),
        }];
        assert!(BookMissingDatePublishedValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_service_provider_v8_missing() {
        let mut p = make_page("https://example.com/svc");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Service".into()),
            data: serde_json::json!({"@type": "Service"}),
        }];
        let f = ServiceMissingProviderValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "SVCPRV-V2001");
    }
    #[test]
    fn test_service_provider_v8_present() {
        let mut p = make_page("https://example.com/svc");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Service".into()),
            data: serde_json::json!({"@type": "Service", "provider": "ACME"}),
        }];
        assert!(ServiceMissingProviderValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_healthplan_provider_v8_missing() {
        let mut p = make_page("https://example.com/hp");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HealthPlan".into()),
            data: serde_json::json!({"@type": "HealthPlan"}),
        }];
        let f = HealthPlanMissingProviderValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HPPRV-V2001");
    }
    #[test]
    fn test_healthplan_provider_v8_present() {
        let mut p = make_page("https://example.com/hp");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("HealthPlan".into()),
            data: serde_json::json!({"@type": "HealthPlan", "provider": "Insurer"}),
        }];
        assert!(HealthPlanMissingProviderValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_invoice_account_v8_missing() {
        let mut p = make_page("https://example.com/inv");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Invoice".into()),
            data: serde_json::json!({"@type": "Invoice"}),
        }];
        let f = InvoiceMissingAccountValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "INVACCT-V2001");
    }
    #[test]
    fn test_invoice_account_v8_present() {
        let mut p = make_page("https://example.com/inv");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Invoice".into()),
            data: serde_json::json!({"@type": "Invoice", "accountId": "123"}),
        }];
        assert!(InvoiceMissingAccountValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_permit_number_v8_missing() {
        let mut p = make_page("https://example.com/perm");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Permit".into()),
            data: serde_json::json!({"@type": "Permit"}),
        }];
        let f = PermitMissingPermitNumberValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PERMNUM-V2001");
    }
    #[test]
    fn test_permit_number_v8_present() {
        let mut p = make_page("https://example.com/perm");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Permit".into()),
            data: serde_json::json!({"@type": "Permit", "permitNumber": "P-123"}),
        }];
        assert!(PermitMissingPermitNumberValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_plan_desc_v8_missing() {
        let mut p = make_page("https://example.com/plan");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Plan".into()),
            data: serde_json::json!({"@type": "Plan"}),
        }];
        let f = PlanMissingDescriptionValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PLANDESC-V2001");
    }
    #[test]
    fn test_plan_desc_v8_present() {
        let mut p = make_page("https://example.com/plan");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Plan".into()),
            data: serde_json::json!({"@type": "Plan", "description": "A plan"}),
        }];
        assert!(PlanMissingDescriptionValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_rp_about_v8_missing() {
        let mut p = make_page("https://example.com/rp");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("ResearchProject".into()),
            data: serde_json::json!({"@type": "ResearchProject"}),
        }];
        let f = ResearchProjectMissingAboutValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "RPABOUT-V2001");
    }
    #[test]
    fn test_rp_about_v8_present() {
        let mut p = make_page("https://example.com/rp");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("ResearchProject".into()),
            data: serde_json::json!({"@type": "ResearchProject", "about": "AI safety"}),
        }];
        assert!(ResearchProjectMissingAboutValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_sched_tz_v8_missing() {
        let mut p = make_page("https://example.com/sched");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Schedule".into()),
            data: serde_json::json!({"@type": "Schedule"}),
        }];
        let f = ScheduleMissingTimezoneValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "SCHEDTZ-V2001");
    }
    #[test]
    fn test_sched_tz_v8_present() {
        let mut p = make_page("https://example.com/sched");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Schedule".into()),
            data: serde_json::json!({"@type": "Schedule", "timezone": "America/New_York"}),
        }];
        assert!(ScheduleMissingTimezoneValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_trip_it_v8_missing() {
        let mut p = make_page("https://example.com/trip");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Trip".into()),
            data: serde_json::json!({"@type": "Trip"}),
        }];
        let f = TripMissingItineraryValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "TRIPIT-V2001");
    }
    #[test]
    fn test_trip_it_v8_present() {
        let mut p = make_page("https://example.com/trip");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Trip".into()),
            data: serde_json::json!({"@type": "Trip", "itinerary": []}),
        }];
        assert!(TripMissingItineraryValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_wu_name_v8_missing() {
        let mut p = make_page("https://example.com/wu");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WorkersUnion".into()),
            data: serde_json::json!({"@type": "WorkersUnion"}),
        }];
        let f = WorkersUnionMissingNameValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WUNAME-V2001");
    }
    #[test]
    fn test_wu_name_v8_present() {
        let mut p = make_page("https://example.com/wu");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WorkersUnion".into()),
            data: serde_json::json!({"@type": "WorkersUnion", "name": "Local 123"}),
        }];
        assert!(WorkersUnionMissingNameValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_webapi_doc_v8_missing() {
        let mut p = make_page("https://example.com/api");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebAPI".into()),
            data: serde_json::json!({"@type": "WebAPI"}),
        }];
        let f = WebAPIMissingDocumentationValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WAPIDOC-V2001");
    }
    #[test]
    fn test_webapi_doc_v8_present() {
        let mut p = make_page("https://example.com/api");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebAPI".into()),
            data: serde_json::json!({"@type": "WebAPI", "documentation": "https://docs.example.com"}),
        }];
        assert!(WebAPIMissingDocumentationValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_wearable_dev_v8_missing() {
        let mut p = make_page("https://example.com/w");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Wearable".into()),
            data: serde_json::json!({"@type": "Wearable"}),
        }];
        let f = WearableMissingDeviceTypeValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WEARDEV-V2001");
    }
    #[test]
    fn test_wearable_dev_v8_present() {
        let mut p = make_page("https://example.com/w");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Wearable".into()),
            data: serde_json::json!({"@type": "Wearable", "deviceType": "SmartWatch"}),
        }];
        assert!(WearableMissingDeviceTypeValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_wpe_name_v8_missing() {
        let mut p = make_page("https://example.com/wpe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebPageElement".into()),
            data: serde_json::json!({"@type": "WebPageElement"}),
        }];
        let f = WebPageElementMissingNameValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WPELNAME-V2001");
    }
    #[test]
    fn test_wpe_name_v8_present() {
        let mut p = make_page("https://example.com/wpe");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("WebPageElement".into()),
            data: serde_json::json!({"@type": "WebPageElement", "name": "Header"}),
        }];
        assert!(WebPageElementMissingNameValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_worker_job_v8_missing() {
        let mut p = make_page("https://example.com/w");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Worker".into()),
            data: serde_json::json!({"@type": "Worker"}),
        }];
        let f = WorkerMissingJobTitleValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "WORKJOB-V2001");
    }
    #[test]
    fn test_worker_job_v8_present() {
        let mut p = make_page("https://example.com/w");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Worker".into()),
            data: serde_json::json!({"@type": "Worker", "jobTitle": "Engineer"}),
        }];
        assert!(WorkerMissingJobTitleValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_org_name_v8_missing() {
        let mut p = make_page("https://example.com/org");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"@type": "Organization"}),
        }];
        let f = OrganizationMissingNameValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "ORGNAME-V2001");
    }
    #[test]
    fn test_org_name_v8_present() {
        let mut p = make_page("https://example.com/org");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Organization".into()),
            data: serde_json::json!({"@type": "Organization", "name": "ACME"}),
        }];
        assert!(OrganizationMissingNameValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_person_name_v8_missing() {
        let mut p = make_page("https://example.com/p");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Person".into()),
            data: serde_json::json!({"@type": "Person"}),
        }];
        let f = PersonMissingNameValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PERSNAME-V2001");
    }
    #[test]
    fn test_person_name_v8_present() {
        let mut p = make_page("https://example.com/p");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Person".into()),
            data: serde_json::json!({"@type": "Person", "name": "John"}),
        }];
        assert!(PersonMissingNameValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_job_title_v8_missing() {
        let mut p = make_page("https://example.com/job");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("JobPosting".into()),
            data: serde_json::json!({"@type": "JobPosting"}),
        }];
        let f = JobPostingMissingTitleValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "JOBTITLE-V2001");
    }
    #[test]
    fn test_job_title_v8_present() {
        let mut p = make_page("https://example.com/job");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("JobPosting".into()),
            data: serde_json::json!({"@type": "JobPosting", "title": "Engineer"}),
        }];
        assert!(JobPostingMissingTitleValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_course_name_v8_missing() {
        let mut p = make_page("https://example.com/c");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Course".into()),
            data: serde_json::json!({"@type": "Course"}),
        }];
        let f = CourseMissingNameValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "COURSENAME-V2001");
    }
    #[test]
    fn test_course_name_v8_present() {
        let mut p = make_page("https://example.com/c");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Course".into()),
            data: serde_json::json!({"@type": "Course", "name": "Rust 101"}),
        }];
        assert!(CourseMissingNameValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_recipe_name_v8_missing() {
        let mut p = make_page("https://example.com/r");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe"}),
        }];
        let f = RecipeMissingNameValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "RECIPENAME-V2001");
    }
    #[test]
    fn test_recipe_name_v8_present() {
        let mut p = make_page("https://example.com/r");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("Recipe".into()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta"}),
        }];
        assert!(RecipeMissingNameValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_lb_npi_v8_missing() {
        let mut p = make_page("https://example.com/lb");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LocalBusiness".into()),
            data: serde_json::json!({"@type": "LocalBusiness"}),
        }];
        let f = LocalBusinessMissingNpiValidatorV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LBPI-V2001");
    }
    #[test]
    fn test_lb_npi_v8_present() {
        let mut p = make_page("https://example.com/lb");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some("LocalBusiness".into()),
            data: serde_json::json!({"@type": "LocalBusiness", "identifier": "12345"}),
        }];
        assert!(LocalBusinessMissingNpiValidatorV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V8 Security Validators (25) tests =====

    // ---------------------------------------------------------------------------
    // Regression tests: severity calibration for missing-property findings.
    // Rule: schema-invalid without it => Error; Google rich result breaks =>
    // Warning; recommended / nice-to-have => Info.
    // ---------------------------------------------------------------------------

    fn one_sd(r#type: &str, data: serde_json::Value) -> crate::parser::ParsedPage {
        let mut p = make_page("https://example.com/sd");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".into()),
            r#type: Some(r#type.to_string()),
            data,
        }];
        p
    }

    #[test]
    fn test_jobposting_valid_through_missing_is_info() {
        let p = one_sd("JobPosting", serde_json::json!({"@type": "JobPosting"}));
        let f = JobPostingValidThroughValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "JOBVT-V5001");
        assert_eq!(f[0].severity, Severity::Info);
    }
    #[test]
    fn test_event_organizer_missing_is_info() {
        let p = one_sd("Event", serde_json::json!({"@type": "Event"}));
        let f = EventOrganizerValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert!(f.iter().all(|x| x.severity == Severity::Info));
    }
    #[test]
    fn test_event_start_date_missing_stays_warning() {
        let p = one_sd("Event", serde_json::json!({"@type": "Event"}));
        let f = EventMissingStartDateValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warning);
    }
    #[test]
    fn test_book_author_v8_missing_is_info() {
        let p = one_sd("Book", serde_json::json!({"@type": "Book"}));
        let f = BookMissingAuthorValidatorV2::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }
    #[test]
    fn test_movie_director_v8_missing_is_info() {
        let p = one_sd("Movie", serde_json::json!({"@type": "Movie"}));
        let f = MovieMissingDirectorValidatorV2::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }
    #[test]
    fn test_dataset_distribution_missing_is_info_but_description_warning() {
        let p = one_sd("Dataset", serde_json::json!({"@type": "Dataset"}));
        let dist = DatasetMissingDistributionValidator::new().analyze(&make_ctx(&p, None));
        assert!(!dist.is_empty());
        assert!(dist.iter().all(|x| x.severity == Severity::Info));
        let desc = DatasetMissingDescriptionValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(desc.len(), 1);
        assert_eq!(desc[0].severity, Severity::Warning);
    }
    #[test]
    fn test_worker_job_title_v8_missing_is_info() {
        let p = one_sd("Worker", serde_json::json!({"@type": "Worker"}));
        let f = WorkerMissingJobTitleValidatorV2::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }
}
